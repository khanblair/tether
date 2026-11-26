package app.tether.session

import app.tether.core.generated.CoreEventView
import app.tether.core.generated.EventSink
import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.ProviderView
import app.tether.core.generated.RewindScopeView
import app.tether.core.generated.SessionRunner
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.SessionRuntimeException
import app.tether.core.generated.SupportView

/** The permission modes, in the order the mode sheet lists them. */
enum class SessionMode { PLAN, ASK, ACCEPT_EDITS, AUTO, BYPASS }

/** What a rewind restores. The file scopes touch the workspace; the conversation scope does not. */
enum class RewindScope { FILES_AND_CONVERSATION, FILES, CONVERSATION }

/** One session as the sheets and the top bar need it. */
data class SessionDetails(
    val title: String,
    val provider: ProviderView,
    val modelId: String,
    val mode: SessionMode,
)

/** One model of a provider. A null capability or price is unknown, not absent. */
data class ModelChoice(
    val id: String,
    val displayName: String,
    val contextWindow: Long?,
    val inputMicrosPerMillion: Long?,
    val outputMicrosPerMillion: Long?,
    val tools: Boolean?,
    val images: Boolean?,
    val reasoning: Boolean?,
)

/** Tokens and cost a session has used so far. */
data class SessionUsage(
    val tokensIn: Long,
    val tokensOut: Long,
    val costMicros: Long,
)

/** A saved restore point, with the workspace paths it captured. */
data class CheckpointItem(
    val id: String,
    val createdAtMs: Long,
    val paths: List<String>,
)

/** One search hit. For a session hit, [matchText] is the session title. */
data class SearchHit(
    val sessionId: String,
    val sessionTitle: String,
    val matchText: String,
)

/** Why a core call failed, in terms the screens can act on. */
enum class SessionFailure {
    BUSY,
    NOT_FOUND,
    EMPTY_INPUT,
    CORRUPT_SNAPSHOT,
    INVALID_PATH,
    UNAVAILABLE,
    OTHER,
}

/** A core call that failed. Its [failure] says why, and [cause] is the core's own error. */
class SessionSettingsException(
    val failure: SessionFailure,
    cause: Throwable? = null,
) : Exception(failure.name, cause)

/**
 * Session settings, catalog, usage, checkpoints, and search from the Rust core. Every call blocks,
 * so callers run it off the main thread. A core error arrives as a [SessionSettingsException].
 * A fake implements this in unit tests.
 */
interface SessionSettingsGateway {
    /** The title, provider, model, and mode of [sessionId]. */
    fun details(sessionId: String): SessionDetails

    /** The saved models of [provider], in the saved catalog order. */
    fun models(provider: ProviderView): List<ModelChoice>

    /** Sets the provider and model for future turns. The caller reopens the runner after it. */
    fun setModel(
        sessionId: String,
        provider: ProviderView,
        modelId: String,
    )

    /** Sets the saved permission mode. The caller reopens the runner after it. */
    fun setMode(
        sessionId: String,
        mode: SessionMode,
    )

    /** Renames the session. [title] is already trimmed and not blank. */
    fun rename(
        sessionId: String,
        title: String,
    )

    /** Whether in-memory Bypass is on for the open session. False when the session is not open. */
    fun bypassActive(sessionId: String): Boolean

    /** Turns Bypass on for the open session. It is never saved, and it ends with the session. */
    fun enableBypass(sessionId: String)

    /** Turns Bypass off, so the open session returns to its saved mode. */
    fun disableBypass(sessionId: String)

    /** The tokens and cost [sessionId] has used. */
    fun usage(sessionId: String): SessionUsage

    /** The checkpoints of [sessionId], as the core lists them (oldest first). */
    fun checkpoints(sessionId: String): List<CheckpointItem>

    /** Sessions in [projectId] whose title matches [query]. */
    fun searchSessions(
        projectId: String,
        query: String,
    ): List<SearchHit>

    /** Messages across all projects that contain [phrase], at most [limit] hits. */
    fun searchTranscripts(
        phrase: String,
        limit: Int,
    ): List<SearchHit>
}

/** Restores a checkpoint through the session's live runner, so the events reach its transcript. */
interface SessionRewinder {
    /** Restores [checkpointId] in [scope]. Blocks until the core has restored it. */
    fun rewind(
        checkpointId: String,
        scope: RewindScope,
    )
}

/**
 * The real settings gateway over the UniFFI [SessionRuntime]. A session's sheets use the runtime of
 * its open connection: the core drops a session's runner from that runtime when the model or mode
 * changes, and only the same runtime can reopen it.
 */
class CoreSessionSettingsGateway(
    private val runtime: Lazy<SessionRuntime>,
) : SessionSettingsGateway {
    override fun details(sessionId: String): SessionDetails =
        coreCall {
            runtime.value.loadSession(sessionId).let { record ->
                SessionDetails(
                    title = record.title,
                    provider = record.provider,
                    modelId = record.modelId,
                    mode = SessionMode.valueOf(record.mode.name),
                )
            }
        }

    override fun models(provider: ProviderView): List<ModelChoice> =
        coreCall {
            runtime.value.listModels(provider).map { model ->
                ModelChoice(
                    id = model.id,
                    displayName = model.displayName,
                    contextWindow = model.contextWindow?.toLong(),
                    inputMicrosPerMillion = model.inputPriceMicrosPerMillion?.toLong(),
                    outputMicrosPerMillion = model.outputPriceMicrosPerMillion?.toLong(),
                    tools = model.tools.flag(),
                    images = model.images.flag(),
                    reasoning = model.reasoning.flag(),
                )
            }
        }

    override fun setModel(
        sessionId: String,
        provider: ProviderView,
        modelId: String,
    ) = coreCall { runtime.value.setSessionModel(sessionId, provider, modelId) }

    override fun setMode(
        sessionId: String,
        mode: SessionMode,
    ) = coreCall { runtime.value.setSessionMode(sessionId, PermissionModeView.valueOf(mode.name)) }

    override fun rename(
        sessionId: String,
        title: String,
    ) = coreCall { runtime.value.renameSession(sessionId, title) }

    override fun bypassActive(sessionId: String): Boolean =
        try {
            coreCall { runtime.value.bypassActive(sessionId) }
        } catch (error: SessionSettingsException) {
            if (error.failure == SessionFailure.NOT_FOUND) false else throw error
        }

    override fun enableBypass(sessionId: String) =
        coreCall { runtime.value.enableBypass(sessionId) }

    override fun disableBypass(sessionId: String) =
        coreCall { runtime.value.disableBypass(sessionId) }

    override fun usage(sessionId: String): SessionUsage =
        coreCall {
            runtime.value.sessionUsage(sessionId).let { usage ->
                SessionUsage(
                    tokensIn = usage.tokensIn.toLong(),
                    tokensOut = usage.tokensOut.toLong(),
                    costMicros = usage.costMicros.toLong(),
                )
            }
        }

    override fun checkpoints(sessionId: String): List<CheckpointItem> =
        coreCall {
            runtime.value.listCheckpoints(sessionId).map { checkpoint ->
                CheckpointItem(checkpoint.id, checkpoint.createdAtMs, checkpoint.paths)
            }
        }

    override fun searchSessions(
        projectId: String,
        query: String,
    ): List<SearchHit> =
        coreCall {
            runtime.value.searchSessions(projectId, query).map { session ->
                SearchHit(session.id, session.title, matchText = session.title)
            }
        }

    override fun searchTranscripts(
        phrase: String,
        limit: Int,
    ): List<SearchHit> =
        coreCall {
            runtime.value.searchTranscripts(phrase, limit.toUInt()).map { hit ->
                SearchHit(hit.sessionId, hit.sessionTitle, hit.text)
            }
        }
}

/** Rewinds through the runner a connection holds. [currentRunner] is read at each call. */
class CoreSessionRewinder(
    private val currentRunner: () -> SessionRunner,
) : SessionRewinder {
    override fun rewind(
        checkpointId: String,
        scope: RewindScope,
    ) = coreCall { currentRunner().rewind(checkpointId, scope.toCore()) }
}

/**
 * Runs one core call and turns its error into a [SessionSettingsException]. Other errors pass
 * through unchanged.
 */
internal inline fun <T> coreCall(block: () -> T): T =
    try {
        block()
    } catch (error: SessionRuntimeException) {
        throw SessionSettingsException(error.failure(), error)
    }

private fun SupportView.flag(): Boolean? =
    when (this) {
        SupportView.YES -> true
        SupportView.NO -> false
        SupportView.UNKNOWN -> null
    }

internal fun SessionRuntimeException.failure(): SessionFailure =
    when (this) {
        is SessionRuntimeException.NotFound -> SessionFailure.NOT_FOUND
        is SessionRuntimeException.EmptyInput -> SessionFailure.EMPTY_INPUT
        is SessionRuntimeException.Busy -> SessionFailure.BUSY
        is SessionRuntimeException.CorruptSnapshot -> SessionFailure.CORRUPT_SNAPSHOT
        is SessionRuntimeException.InvalidPath -> SessionFailure.INVALID_PATH
        is SessionRuntimeException.MissingKey, is SessionRuntimeException.Unavailable ->
            SessionFailure.UNAVAILABLE
        // These come from the plan, bypass, and instructions calls, which have no screen yet.
        is SessionRuntimeException.BypassNeedsConfirmation,
        is SessionRuntimeException.InstructionsTooLong,
        is SessionRuntimeException.InvalidWorkingMode,
        -> SessionFailure.OTHER
    }

private fun RewindScope.toCore(): RewindScopeView =
    when (this) {
        RewindScope.FILES_AND_CONVERSATION -> RewindScopeView.BOTH
        RewindScope.FILES -> RewindScopeView.FILES
        RewindScope.CONVERSATION -> RewindScopeView.CONVERSATION
    }

/** The event sink for settings calls. No turn runs through it, so events are dropped. */
internal object QuietEvents : EventSink {
    override fun onEvent(event: CoreEventView) = Unit
}
