package app.tether.session

import app.tether.core.generated.ProviderView
import app.tether.providers.ProviderGateway
import app.tether.settings.AppDefaultSource
import app.tether.settings.DefaultModel
import app.tether.settings.ProjectDefaultsGateway

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

    /** A gateway call threw. No session was created. */
    data object Failed : StartResult
}

/**
 * Starts a plain chat session in a project. The model comes from the first of these that is set
 * and has a saved key: the project's default, the app default, then the first provider in
 * [ProviderView] order that has a key, with that provider's first saved model. A default whose
 * provider has no key is skipped. Blocking: call it off the main thread.
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
            val chosen =
                savedChoice(projectDefault(projectId)) ?: savedChoice(appDefaults?.defaultModel())
            if (chosen != null) {
                StartResult.Started(
                    sessions.createSession(projectId, chosen.provider, chosen.modelId),
                )
            } else {
                firstKeyedStart(projectId)
            }
        } catch (_: Exception) {
            StartResult.Failed
        }

    private fun firstKeyedStart(projectId: String): StartResult {
        val provider =
            ProviderView.entries.firstOrNull { providers.hasKey(it) } ?: return StartResult.NoKey
        val model = sessions.defaultModel(provider)?.trim()?.takeIf(String::isNotEmpty)
        return if (model == null) {
            StartResult.NoModels
        } else {
            StartResult.Started(sessions.createSession(projectId, provider, model))
        }
    }

    private fun projectDefault(projectId: String): DefaultModel? {
        val saved = projectDefaults?.load(projectId)
        val provider = saved?.provider
        val modelId = saved?.modelId
        return if (provider != null && modelId != null) DefaultModel(provider, modelId) else null
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
