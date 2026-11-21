package app.tether.session

import app.tether.core.generated.ProviderView
import app.tether.providers.ProviderGateway

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
 * Starts a plain chat session in a project. It takes the first provider, in [ProviderView] order,
 * that has a saved key, and that provider's first saved model. Blocking: call it off the main
 * thread.
 */
class SessionStarter(
    private val providers: ProviderGateway,
    private val sessions: SessionGateway,
) {
    /** Creates the session in [projectId], or says why it could not. */
    @Suppress("TooGenericExceptionCaught")
    fun start(projectId: String): StartResult =
        try {
            val provider =
                ProviderView.entries.firstOrNull { providers.hasKey(it) }
                    ?: return StartResult.NoKey
            val model =
                sessions
                    .defaultModel(provider)
                    ?.trim()
                    ?.takeIf(String::isNotEmpty)
                    ?: return StartResult.NoModels
            StartResult.Started(sessions.createSession(projectId, provider, model))
        } catch (_: Exception) {
            StartResult.Failed
        }
}
