package app.tether.session

import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.ProviderView
import app.tether.providers.ProviderGateway
import app.tether.settings.AppDefaultSource
import app.tether.settings.DefaultModel
import app.tether.settings.ProjectDefaults
import app.tether.settings.ProjectDefaultsGateway
import kotlinx.coroutines.CancellationException

/** The outcome of [SessionStarter.start]. */
sealed interface StartResult {
    /** A session was created and is ready to open. */
    data class Started(
        val sessionId: String,
    ) : StartResult

    /** No provider has a saved key. No session was created. */
    data object NoKey : StartResult

    /** The first provider with a key has no saved model. No session was created. */
    data object NoModels : StartResult

    /** A gateway call threw before a session was created. No session was created. */
    data object Failed : StartResult
}

/**
 * Starts a plain chat session in a project. The model comes from the first of these that is set
 * and has a saved key: the project's default, the app default, then the first provider in
 * [ProviderView] order that has a key, with that provider's first saved model. A default whose
 * provider has no key is skipped. The permission mode is the project's default, else the app's;
 * when neither is saved the core's own mode stays. The app's thinking effort is applied when it is
 * not the model default. Blocking: call it off the main thread.
 */
class SessionStarter(
    private val providers: ProviderGateway,
    private val sessions: SessionGateway,
    private val projectDefaults: ProjectDefaultsGateway? = null,
    private val appDefaults: AppDefaultSource? = null,
) {
    /** Creates the session in [projectId], or says why it could not. */
    @Suppress("TooGenericExceptionCaught")
    fun start(projectId: String): StartResult =
        try {
            val project = projectDefaults?.load(projectId)
            val mode = project?.mode ?: appDefaults?.savedDefaultMode()
            val effort = appDefaults?.savedDefaultEffort()
            val chosen = savedChoice(project?.model()) ?: savedChoice(appDefaults?.defaultModel())
            if (chosen != null) {
                createStarted(projectId, chosen.provider, chosen.modelId, mode, effort)
            } else {
                firstKeyedStart(projectId, mode, effort)
            }
        } catch (_: Exception) {
            StartResult.Failed
        }

    private fun firstKeyedStart(
        projectId: String,
        mode: SessionMode?,
        effort: ThinkingEffort?,
    ): StartResult {
        val provider =
            ProviderView.entries.firstOrNull { providers.hasKey(it) } ?: return StartResult.NoKey
        val model = sessions.defaultModel(provider)?.trim()?.takeIf(String::isNotEmpty)
        return if (model == null) {
            StartResult.NoModels
        } else {
            createStarted(projectId, provider, model, mode, effort)
        }
    }

    /**
     * Creates the session, then applies [mode] and [effort]. Bypass is never applied from a saved
     * default, so the core's own mode stays. A null [effort] is the model default and makes no
     * call. A refused mode or effort call does not fail the start: the session still exists.
     */
    private fun createStarted(
        projectId: String,
        provider: ProviderView,
        modelId: String,
        mode: SessionMode?,
        effort: ThinkingEffort?,
    ): StartResult {
        val sessionId = sessions.createSession(projectId, provider, modelId)
        if (mode != null && mode != SessionMode.BYPASS) applyMode(sessionId, mode)
        if (effort != null) applyEffort(sessionId, effort)
        return StartResult.Started(sessionId)
    }

    /** Sets [effort] on a new session. A refused call is ignored, and the model default stays. */
    @Suppress("TooGenericExceptionCaught")
    private fun applyEffort(
        sessionId: String,
        effort: ThinkingEffort,
    ) {
        try {
            sessions.setEffort(sessionId, effort)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            // The session keeps the model's own default effort.
        }
    }

    /** Sets [mode] on a new session. A refused call is ignored, and the core's mode stays. */
    @Suppress("TooGenericExceptionCaught")
    private fun applyMode(
        sessionId: String,
        mode: SessionMode,
    ) {
        try {
            sessions.setMode(sessionId, PermissionModeView.valueOf(mode.name))
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            // The core keeps its own default mode, which is the safest one to run in.
        }
    }

    /**
     * The saved model, with its id trimmed, when its provider has a key and the id is not blank.
     */
    private fun savedChoice(model: DefaultModel?): DefaultModel? {
        val id = model?.modelId?.trim() ?: return null
        return if (id.isNotEmpty() && providers.hasKey(model.provider)) {
            DefaultModel(model.provider, id)
        } else {
            null
        }
    }
}

/** The project's model as one value, or null when either half is missing. */
private fun ProjectDefaults.model(): DefaultModel? =
    if (provider != null && modelId != null) DefaultModel(provider, modelId) else null
