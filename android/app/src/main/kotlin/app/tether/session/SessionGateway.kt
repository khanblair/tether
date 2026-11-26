package app.tether.session

import app.tether.core.generated.CoreEventView
import app.tether.core.generated.EventSink
import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.ProviderView
import app.tether.core.generated.SecretVault
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.SessionStatusView

/** One saved session as the project and home screens use it. */
data class SessionInfo(
    val id: String,
    val title: String,
    val status: SessionStatusView,
    val updatedAtMs: Long,
)

/**
 * What the session-list screens need from the Rust core. Every call blocks, so callers run it off
 * the main thread. A fake implements this in unit tests.
 */
interface SessionGateway {
    /** The sessions of [projectId], newest first, as the core orders them. */
    fun listSessions(projectId: String): List<SessionInfo>

    /** The first model in [provider]'s saved list, or null when the list is empty. */
    fun defaultModel(provider: ProviderView): String?

    /** Creates a plain chat session in [projectId] and returns its id. */
    fun createSession(
        projectId: String,
        provider: ProviderView,
        modelId: String,
    ): String

    /** Sets the permission mode of [sessionId]. Used right after creation, before any turn. */
    fun setMode(
        sessionId: String,
        mode: PermissionModeView,
    )

    /** Sets the thinking effort of [sessionId], or clears it with null. Used after creation. */
    fun setEffort(
        sessionId: String,
        effort: ThinkingEffort?,
    )

    /** Renames [sessionId] to [title]. The caller trims the title and never sends a blank one. */
    fun rename(
        sessionId: String,
        title: String,
    )
}

/**
 * The real gateway over the UniFFI [SessionRuntime]. The runtime opens on the first call, which
 * runs off the main thread. Lists and creation need no events, so the sink drops them.
 */
class CoreSessionGateway(
    private val databasePath: String,
    private val vault: SecretVault,
) : SessionGateway {
    private val runtime: SessionRuntime by lazy {
        SessionRuntime(databasePath, vault, DroppedEvents)
    }

    override fun listSessions(projectId: String): List<SessionInfo> =
        runtime.listSessions(projectId).map { record ->
            SessionInfo(
                id = record.id,
                title = record.title,
                status = record.status,
                updatedAtMs = record.updatedAtMs,
            )
        }

    override fun defaultModel(provider: ProviderView): String? = runtime.defaultModel(provider)

    override fun createSession(
        projectId: String,
        provider: ProviderView,
        modelId: String,
    ): String = runtime.createSession(projectId, provider, modelId).id

    override fun setMode(
        sessionId: String,
        mode: PermissionModeView,
    ) = runtime.setSessionMode(sessionId, mode)

    override fun setEffort(
        sessionId: String,
        effort: ThinkingEffort?,
    ) = coreCall { runtime.setSessionReasoning(sessionId, effort?.toCore()) }

    override fun rename(
        sessionId: String,
        title: String,
    ) = coreCall { runtime.renameSession(sessionId, title) }
}

/** The event sink for list and create calls. No turn runs through it, so events are dropped. */
private object DroppedEvents : EventSink {
    override fun onEvent(event: CoreEventView) = Unit
}
