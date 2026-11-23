package app.tether.session.agent

import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.SessionRuntimeException

/** The step the core says an interrupted turn stopped at, and when it stopped. */
data class InterruptionUi(
    val step: String,
    val atMs: Long,
)

/** Why an agent-loop call failed, in terms the screens can act on. */
enum class AgentLoopFailure {
    /** The session, call, or question is unknown or already answered. */
    NotFound,

    /** The session is busy with another operation. */
    Busy,

    /** The core or its store is not available, or a provider key is missing. */
    Unavailable,

    /** Any other core error, such as bypass that needs confirmation. */
    Other,
}

/** An agent-loop call that failed. [reason] says why, and [cause] is the core's own error. */
class AgentLoopException(
    val reason: AgentLoopFailure,
    cause: Throwable? = null,
) : Exception(reason.name, cause)

/**
 * The agent-loop calls of the Rust core. Every call blocks, so callers run it off the main
 * thread. A core error arrives as an [AgentLoopException]. A fake implements this in unit tests.
 */
interface AgentLoopGateway {
    /** The approvals waiting for [sessionId]. */
    fun pendingApprovals(sessionId: String): List<ApprovalUi>

    /** Allows the pending call [toolCallId]. [always] also allows future calls of the same kind. */
    fun approve(
        sessionId: String,
        toolCallId: String,
        always: Boolean,
    )

    /** Denies the pending call [toolCallId]. [reason] is shown to the agent when it is set. */
    fun deny(
        sessionId: String,
        toolCallId: String,
        reason: String?,
    )

    /** The question waiting for [sessionId], or null when none waits. */
    fun pendingQuestion(sessionId: String): QuestionUi?

    /** Answers the question of [toolCallId] with [answer]. */
    fun answerQuestion(
        sessionId: String,
        toolCallId: String,
        answer: String,
    )

    /** The checklist of [sessionId]. */
    fun todos(sessionId: String): List<TodoUi>

    /** The plan waiting for [sessionId], or null when none waits. */
    fun pendingPlan(sessionId: String): PlanUi?

    /** Runs the pending plan in [workingMode]. The core refuses Plan and Bypass. */
    fun runPlan(
        sessionId: String,
        workingMode: PermissionModeView,
    )

    /** Lets [sessionId] go on past a budget stop. */
    fun continuePastBudget(sessionId: String)

    /** The interrupted step of [sessionId], or null when no turn was interrupted. */
    fun interruption(sessionId: String): InterruptionUi?

    /** Resumes the interrupted turn of [sessionId]. */
    fun resume(sessionId: String)

    /** Drops the interrupted turn of [sessionId] without resuming it. */
    fun discardInterrupted(sessionId: String)

    /** Turns on Bypass for the open session [sessionId], in memory only. */
    fun enableBypass(sessionId: String)

    /** Turns off Bypass for the open session [sessionId]. */
    fun disableBypass(sessionId: String)
}

/**
 * The real agent-loop gateway over the UniFFI [SessionRuntime]. It uses the runtime of the open
 * connection, because only that runtime holds the session's pending calls.
 */
class CoreAgentLoopGateway(
    private val runtime: Lazy<SessionRuntime>,
) : AgentLoopGateway {
    override fun pendingApprovals(sessionId: String): List<ApprovalUi> =
        agentCall { runtime.value.pendingApprovals(sessionId).map { it.toApprovalUi() } }

    override fun approve(
        sessionId: String,
        toolCallId: String,
        always: Boolean,
    ) = agentCall { runtime.value.approve(sessionId, toolCallId, always) }

    override fun deny(
        sessionId: String,
        toolCallId: String,
        reason: String?,
    ) = agentCall { runtime.value.deny(sessionId, toolCallId, reason) }

    override fun pendingQuestion(sessionId: String): QuestionUi? =
        agentCall {
            runtime.value
                .pendingQuestions(sessionId)
                .firstOrNull()
                ?.toQuestionUi()
        }

    override fun answerQuestion(
        sessionId: String,
        toolCallId: String,
        answer: String,
    ) = agentCall { runtime.value.answerQuestion(sessionId, toolCallId, answer) }

    override fun todos(sessionId: String): List<TodoUi> =
        agentCall { runtime.value.todos(sessionId).map { it.toTodoUi() } }

    override fun pendingPlan(sessionId: String): PlanUi? =
        agentCall { runtime.value.pendingPlan(sessionId)?.toPlanUi(toolCallId = null) }

    override fun runPlan(
        sessionId: String,
        workingMode: PermissionModeView,
    ) = agentCall { runtime.value.runPlan(sessionId, workingMode) }

    override fun continuePastBudget(sessionId: String) =
        agentCall { runtime.value.continuePastBudget(sessionId) }

    override fun interruption(sessionId: String): InterruptionUi? =
        agentCall { runtime.value.interruption(sessionId)?.toInterruptionUi() }

    override fun resume(sessionId: String) = agentCall { runtime.value.resumeSession(sessionId) }

    override fun discardInterrupted(sessionId: String) =
        agentCall { runtime.value.discardInterrupted(sessionId) }

    override fun enableBypass(sessionId: String) =
        agentCall { runtime.value.enableBypass(sessionId) }

    override fun disableBypass(sessionId: String) =
        agentCall { runtime.value.disableBypass(sessionId) }
}

/** Runs one core call and turns its error into an [AgentLoopException]. */
private inline fun <T> agentCall(block: () -> T): T =
    try {
        block()
    } catch (error: SessionRuntimeException) {
        throw AgentLoopException(error.agentFailure(), error)
    }

private fun SessionRuntimeException.agentFailure(): AgentLoopFailure =
    when (this) {
        is SessionRuntimeException.NotFound -> AgentLoopFailure.NotFound
        is SessionRuntimeException.Busy -> AgentLoopFailure.Busy
        is SessionRuntimeException.MissingKey,
        is SessionRuntimeException.Unavailable,
        -> AgentLoopFailure.Unavailable
        is SessionRuntimeException.EmptyInput,
        is SessionRuntimeException.CorruptSnapshot,
        is SessionRuntimeException.InvalidPath,
        is SessionRuntimeException.BypassNeedsConfirmation,
        is SessionRuntimeException.InstructionsTooLong,
        is SessionRuntimeException.InvalidWorkingMode,
        -> AgentLoopFailure.Other
    }
