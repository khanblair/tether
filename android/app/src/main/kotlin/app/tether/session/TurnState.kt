package app.tether.session

import app.tether.core.generated.CoreErrorView
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.SessionStatusView

/** The wait after which the thinking label shows elapsed seconds. */
internal const val THINKING_SECONDS_DELAY_MS = 5_000L
private const val MS_PER_SECOND = 1_000L

/** A turn that stopped before it finished. [proposedTool] is the tool it was running, if any. */
data class StoppedTurn(
    val proposedTool: Pair<String, String>?,
)

/**
 * What the transcript shows about the current turn. [requestingSinceMs] is set while the core
 * waits for the first token. [failure] is the core's error for a turn that ended in one.
 */
data class TurnState(
    val running: Boolean = false,
    val proposedTool: Pair<String, String>? = null,
    val requestingSinceMs: Long? = null,
    val failure: CoreErrorView? = null,
    val stopped: StoppedTurn? = null,
    val sendFailed: Boolean = false,
) {
    /** True when the transcript shows a block below the messages. */
    val hasFooter: Boolean
        get() = failure != null || stopped != null || requestingSinceMs != null || sendFailed

    /** A message was sent: the turn runs and every earlier problem clears. */
    fun onSendStarted(): TurnState = TurnState(running = true)

    /** The core refused the send, so no turn started. */
    fun onSendFailed(): TurnState = copy(running = false, sendFailed = true)

    /** The user dismissed the stopped block. */
    fun onDismissStopped(): TurnState = copy(stopped = null)

    /** Applies one core event. [nowMs] stamps the start of a first-token wait. */
    fun onEvent(
        event: CoreEventView,
        nowMs: Long,
    ): TurnState =
        when (event) {
            is CoreEventView.ToolProposed -> copy(proposedTool = event.tool to event.target)
            is CoreEventView.SessionUpdated -> onStatus(event.status, nowMs)
            is CoreEventView.TurnFinished -> copy(running = false, requestingSinceMs = null)
            is CoreEventView.TurnFailed ->
                copy(running = false, requestingSinceMs = null, failure = event.error)
            is CoreEventView.TurnInterrupted -> stopTurn()
            else -> this
        }

    private fun onStatus(
        status: SessionStatusView,
        nowMs: Long,
    ): TurnState =
        when (status) {
            SessionStatusView.REQUESTING -> copy(requestingSinceMs = requestingSinceMs ?: nowMs)
            // A stop the system reports after the turn already ended is not a new stop.
            SessionStatusView.INTERRUPTED -> if (running) stopTurn() else this
            else -> copy(requestingSinceMs = null)
        }

    private fun stopTurn(): TurnState =
        copy(running = false, requestingSinceMs = null, stopped = StoppedTurn(proposedTool))
}

/**
 * The whole seconds to show in the thinking label, or null before [THINKING_SECONDS_DELAY_MS]
 * has passed since [startedAtMs].
 */
internal fun thinkingSeconds(
    startedAtMs: Long,
    nowMs: Long,
): Long? {
    val elapsedMs = nowMs - startedAtMs
    return if (elapsedMs < THINKING_SECONDS_DELAY_MS) null else elapsedMs / MS_PER_SECOND
}
