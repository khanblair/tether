package app.tether.session.agent

import app.tether.core.generated.CoreEventView

/** Output kept per tool call, in characters. Older text is cut from the front. */
internal const val MAX_OUTPUT_CHARS = 20_000

/** Quality reports kept, newest last. */
internal const val MAX_QUALITY_REPORTS = 20

/**
 * The agent-loop state of one session screen: tool calls, approvals, the question, the checklist,
 * the plan, quality reports, banners, and usage. [onEvent] is a pure reducer. The caller filters
 * events of other sessions before it calls [onEvent].
 */
data class AgentLoopState(
    val calls: List<ToolCallUi> = emptyList(),
    val approvals: List<ApprovalUi> = emptyList(),
    val question: QuestionUi? = null,
    val todos: List<TodoUi> = emptyList(),
    val plan: PlanUi? = null,
    val quality: List<QualityUi> = emptyList(),
    val banners: List<AgentBanner> = emptyList(),
    val usage: UsageUi? = null,
) {
    /**
     * Applies one core event and returns the new state. Events this state does not track return
     * the state unchanged.
     */
    fun onEvent(event: CoreEventView): AgentLoopState =
        when (event) {
            is CoreEventView.ToolProposed,
            is CoreEventView.ApprovalRequired,
            is CoreEventView.ToolStarted,
            is CoreEventView.ToolOutput,
            is CoreEventView.ToolFinished,
            -> onToolEvent(event)
            is CoreEventView.BudgetWarning,
            is CoreEventView.BudgetReached,
            is CoreEventView.NetworkPaused,
            is CoreEventView.NetworkResumed,
            is CoreEventView.ContextCompacted,
            -> onNoticeEvent(event)
            is CoreEventView.TurnFinished,
            is CoreEventView.TurnFailed,
            is CoreEventView.TurnInterrupted,
            -> copy(approvals = emptyList(), question = null)
            is CoreEventView.TodoUpdated -> copy(todos = event.items.map { it.toTodoUi() })
            is CoreEventView.QuestionAsked -> copy(question = event.toQuestionUi())
            is CoreEventView.PlanReady -> copy(plan = event.plan.toPlanUi(event.toolCallId))
            is CoreEventView.UsageUpdated -> copy(usage = event.usage.toUsageUi())
            is CoreEventView.QualityReport -> addQuality(event.toQualityUi())
            else -> this
        }

    private fun onToolEvent(event: CoreEventView): AgentLoopState =
        when (event) {
            is CoreEventView.ToolProposed -> proposeCall(event)
            is CoreEventView.ApprovalRequired -> requireApproval(event)
            is CoreEventView.ToolStarted ->
                setCallStatus(event.toolCallId, ToolStatus.Running)
                    .withoutApproval(event.toolCallId)
            is CoreEventView.ToolOutput -> appendOutput(event.toolCallId, event.chunk)
            is CoreEventView.ToolFinished ->
                setCallStatus(event.toolCallId, event.status.toToolStatus())
                    .withoutApproval(event.toolCallId)
            else -> this
        }

    private fun onNoticeEvent(event: CoreEventView): AgentLoopState =
        when (event) {
            is CoreEventView.BudgetWarning -> onBudgetWarning(event)
            is CoreEventView.BudgetReached -> onBudgetReached(event)
            is CoreEventView.NetworkPaused -> addNetworkPause()
            is CoreEventView.NetworkResumed ->
                copy(banners = banners.filterNot { it == AgentBanner.NetworkPaused })
            is CoreEventView.ContextCompacted -> onContextCompacted(event)
            else -> this
        }

    /**
     * Clears what belongs to the last turn: calls, approvals, the question, quality reports, and
     * banners, except the latest compaction notice. Todos, the plan, and usage stay.
     */
    fun onNewTurn(): AgentLoopState =
        copy(
            calls = emptyList(),
            approvals = emptyList(),
            question = null,
            quality = emptyList(),
            banners = banners.filterIsInstance<AgentBanner.Compacted>(),
        )

    /** Removes the approval of [toolCallId] after the user answered it. */
    fun onApprovalResolved(toolCallId: String): AgentLoopState = withoutApproval(toolCallId)

    /** Clears the question after the user answered it. */
    fun onQuestionAnswered(): AgentLoopState = copy(question = null)

    /** Clears the plan after the user ran it or dismissed it. */
    fun onPlanHandled(): AgentLoopState = copy(plan = null)

    /** Removes the budget stop after the user chose to continue past it. */
    fun onBudgetContinued(): AgentLoopState =
        copy(banners = banners.filterNot { it is AgentBanner.BudgetReached })

    /**
     * Returns the state after an app restart. The approvals, question, checklist, and plan come
     * from the core. Each approval also gets a call in [AgentLoopState.calls], so its card shows.
     */
    fun hydrate(
        approvals: List<ApprovalUi>,
        question: QuestionUi?,
        todos: List<TodoUi>,
        plan: PlanUi?,
    ): AgentLoopState {
        val unique = approvals.distinctBy { it.toolCallId }
        val restored =
            unique
                .filter { approval -> calls.none { it.id == approval.toolCallId } }
                .map { it.toCall(ToolStatus.AwaitingApproval) }
        return copy(
            calls = calls + restored,
            approvals = unique,
            question = question,
            todos = todos,
            plan = plan,
        )
    }

    private fun proposeCall(event: CoreEventView.ToolProposed): AgentLoopState =
        if (calls.any { it.id == event.toolCallId }) {
            this
        } else {
            copy(
                calls =
                    calls +
                        ToolCallUi(
                            event.toolCallId,
                            event.tool,
                            event.target,
                            ToolStatus.Proposed,
                            "",
                            false,
                        ),
            )
        }

    private fun requireApproval(event: CoreEventView.ApprovalRequired): AgentLoopState {
        val known = calls.any { it.id == event.toolCallId }
        val newCalls =
            if (known) {
                setCallStatus(event.toolCallId, ToolStatus.AwaitingApproval).calls
            } else {
                calls +
                    ToolCallUi(
                        event.toolCallId,
                        event.tool,
                        event.target,
                        ToolStatus.AwaitingApproval,
                        "",
                        false,
                    )
            }
        val newApprovals =
            if (approvals.any { it.toolCallId == event.toolCallId }) {
                approvals
            } else {
                approvals + ApprovalUi(event.toolCallId, event.tool, event.target)
            }
        return copy(calls = newCalls, approvals = newApprovals)
    }

    private fun setCallStatus(
        toolCallId: String,
        status: ToolStatus,
    ): AgentLoopState =
        copy(calls = calls.map { if (it.id == toolCallId) it.copy(status = status) else it })

    private fun withoutApproval(toolCallId: String): AgentLoopState =
        copy(approvals = approvals.filterNot { it.toolCallId == toolCallId })

    private fun appendOutput(
        toolCallId: String,
        chunk: String,
    ): AgentLoopState =
        copy(calls = calls.map { if (it.id == toolCallId) it.appendChunk(chunk) else it })

    private fun addQuality(report: QualityUi): AgentLoopState =
        copy(quality = (quality + report).takeLast(MAX_QUALITY_REPORTS))

    private fun onBudgetWarning(event: CoreEventView.BudgetWarning): AgentLoopState {
        val scope = event.scope.toBudgetScope()
        val warning =
            AgentBanner.BudgetWarning(
                scope = scope,
                percent = event.percent.toIntClamped(),
                spentMicros = event.spentMicros.toLongClamped(),
                limitMicros = event.limitMicros.toLongClamped(),
            )
        return copy(
            banners =
                banners.filterNot { it is AgentBanner.BudgetWarning && it.scope == scope } +
                    warning,
        )
    }

    private fun onBudgetReached(event: CoreEventView.BudgetReached): AgentLoopState {
        val scope = event.scope.toBudgetScope()
        val reached =
            AgentBanner.BudgetReached(
                scope = scope,
                spentMicros = event.spentMicros.toLongClamped(),
                limitMicros = event.limitMicros.toLongClamped(),
            )
        return copy(
            banners =
                banners.filterNot { it is AgentBanner.BudgetReached && it.scope == scope } +
                    reached,
        )
    }

    private fun addNetworkPause(): AgentLoopState =
        if (AgentBanner.NetworkPaused in banners) {
            this
        } else {
            copy(banners = banners + AgentBanner.NetworkPaused)
        }

    private fun onContextCompacted(event: CoreEventView.ContextCompacted): AgentLoopState {
        val notice =
            AgentBanner.Compacted(
                tokensBefore = event.tokensBefore.toIntClamped(),
                tokensAfter = event.tokensAfter.toIntClamped(),
            )
        return copy(banners = banners.filterNot { it is AgentBanner.Compacted } + notice)
    }
}

/** Keeps the newest [MAX_OUTPUT_CHARS] characters. It never cuts inside a surrogate pair. */
internal fun ToolCallUi.appendChunk(chunk: String): ToolCallUi {
    val joined = output + chunk
    if (joined.length <= MAX_OUTPUT_CHARS) return copy(output = joined)
    val start = joined.length - MAX_OUTPUT_CHARS
    // A low surrogate at the cut has lost its high half, so it goes too.
    val cut = if (joined[start].isLowSurrogate()) start + 1 else start
    return copy(output = joined.substring(cut), outputTruncated = true)
}

private fun ApprovalUi.toCall(status: ToolStatus): ToolCallUi =
    ToolCallUi(toolCallId, tool, target, status, "", false)
