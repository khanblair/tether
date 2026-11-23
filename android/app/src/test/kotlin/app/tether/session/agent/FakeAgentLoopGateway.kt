package app.tether.session.agent

import app.tether.core.generated.PermissionModeView

/**
 * An agent-loop gateway for unit tests. Each call appends one entry to [log], such as
 * "approve:s1,t1,true", and returns the value the test scripted. Set [failNext] to make the next
 * call throw, then it clears itself.
 */
class FakeAgentLoopGateway : AgentLoopGateway {
    val log = mutableListOf<String>()

    var approvals: List<ApprovalUi> = emptyList()
    var question: QuestionUi? = null
    var todos: List<TodoUi> = emptyList()
    var plan: PlanUi? = null
    var interruption: InterruptionUi? = null

    /** The exception the next call throws. It clears after one throw. */
    var failNext: Exception? = null

    private fun record(entry: String) {
        log += entry
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun pendingApprovals(sessionId: String): List<ApprovalUi> {
        record("pendingApprovals:$sessionId")
        return approvals
    }

    override fun approve(
        sessionId: String,
        toolCallId: String,
        always: Boolean,
    ) = record("approve:$sessionId,$toolCallId,$always")

    override fun deny(
        sessionId: String,
        toolCallId: String,
        reason: String?,
    ) = record("deny:$sessionId,$toolCallId,$reason")

    override fun pendingQuestion(sessionId: String): QuestionUi? {
        record("pendingQuestion:$sessionId")
        return question
    }

    override fun answerQuestion(
        sessionId: String,
        toolCallId: String,
        answer: String,
    ) = record("answerQuestion:$sessionId,$toolCallId,$answer")

    override fun todos(sessionId: String): List<TodoUi> {
        record("todos:$sessionId")
        return todos
    }

    override fun pendingPlan(sessionId: String): PlanUi? {
        record("pendingPlan:$sessionId")
        return plan
    }

    override fun runPlan(
        sessionId: String,
        workingMode: PermissionModeView,
    ) = record("runPlan:$sessionId,$workingMode")

    override fun continuePastBudget(sessionId: String) = record("continuePastBudget:$sessionId")

    override fun interruption(sessionId: String): InterruptionUi? {
        record("interruption:$sessionId")
        return interruption
    }

    override fun resume(sessionId: String) = record("resume:$sessionId")

    override fun discardInterrupted(sessionId: String) = record("discardInterrupted:$sessionId")

    override fun enableBypass(sessionId: String) = record("enableBypass:$sessionId")

    override fun disableBypass(sessionId: String) = record("disableBypass:$sessionId")
}
