package app.tether.session.agent

import app.tether.core.generated.BudgetScopeView
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.InterruptionView
import app.tether.core.generated.PendingApprovalView
import app.tether.core.generated.PendingQuestionView
import app.tether.core.generated.PlanView
import app.tether.core.generated.QualityFindingRecordView
import app.tether.core.generated.TodoItemView
import app.tether.core.generated.TodoStatusView
import app.tether.core.generated.ToolCallStatusView
import app.tether.core.generated.UsageSnapshotView

internal fun ToolCallStatusView.toToolStatus(): ToolStatus =
    when (this) {
        ToolCallStatusView.PROPOSED -> ToolStatus.Proposed
        ToolCallStatusView.ALLOWED -> ToolStatus.Allowed
        ToolCallStatusView.AWAITING_APPROVAL -> ToolStatus.AwaitingApproval
        ToolCallStatusView.RUNNING -> ToolStatus.Running
        ToolCallStatusView.DENIED -> ToolStatus.Denied
        ToolCallStatusView.BLOCKED -> ToolStatus.Blocked
        ToolCallStatusView.SUCCEEDED -> ToolStatus.Succeeded
        ToolCallStatusView.FAILED -> ToolStatus.Failed
        ToolCallStatusView.TIMED_OUT -> ToolStatus.TimedOut
        ToolCallStatusView.CANCELLED -> ToolStatus.Cancelled
    }

internal fun TodoStatusView.toTodoStatus(): TodoStatus =
    when (this) {
        TodoStatusView.PENDING -> TodoStatus.Pending
        TodoStatusView.IN_PROGRESS -> TodoStatus.InProgress
        TodoStatusView.COMPLETED -> TodoStatus.Completed
    }

internal fun BudgetScopeView.toBudgetScope(): BudgetScope =
    when (this) {
        BudgetScopeView.SESSION -> BudgetScope.Session
        BudgetScopeView.DAY -> BudgetScope.Day
    }

/** The core sends "info", "warn" or "block". Any other value shows as [FindingSeverity.Info]. */
internal fun String.toFindingSeverity(): FindingSeverity =
    when (this) {
        "warn" -> FindingSeverity.Warn
        "block" -> FindingSeverity.Block
        else -> FindingSeverity.Info
    }

/** Converts a byte count to Int. Values above the Int range are clamped, never wrapped. */
internal fun UByte.toIntClamped(): Int = toInt()

/** Converts an unsigned count to Int. Values above [Int.MAX_VALUE] are clamped. */
internal fun UInt.toIntClamped(): Int =
    if (this > Int.MAX_VALUE.toUInt()) Int.MAX_VALUE else toInt()

/** Converts an unsigned count to Long. Values above [Long.MAX_VALUE] are clamped. */
internal fun ULong.toLongClamped(): Long =
    if (this > Long.MAX_VALUE.toULong()) Long.MAX_VALUE else toLong()

internal fun PendingApprovalView.toApprovalUi(): ApprovalUi = ApprovalUi(toolCallId, tool, target)

internal fun PendingQuestionView.toQuestionUi(): QuestionUi =
    QuestionUi(toolCallId, question, options, allowFreeText)

/** Maps a question the agent asked during the turn. */
internal fun CoreEventView.QuestionAsked.toQuestionUi(): QuestionUi =
    QuestionUi(toolCallId, question, options, allowFreeText)

internal fun TodoItemView.toTodoUi(): TodoUi = TodoUi(text, status.toTodoStatus())

/** Maps a plan. [toolCallId] is null for a plan the core returns without its call. */
internal fun PlanView.toPlanUi(toolCallId: String?): PlanUi =
    PlanUi(toolCallId, summary, steps, files, commands, openQuestions)

internal fun UsageSnapshotView.toUsageUi(): UsageUi =
    UsageUi(tokensIn.toLongClamped(), tokensOut.toLongClamped(), costMicros.toLongClamped())

internal fun InterruptionView.toInterruptionUi(): InterruptionUi = InterruptionUi(step, atMs)

internal fun QualityFindingRecordView.toQualityFindingUi(): QualityFindingUi =
    QualityFindingUi(
        ruleId = ruleId,
        severity = severity.toFindingSeverity(),
        path = path,
        line = line.toIntClamped(),
        message = message,
        advice = advice,
    )

/** Maps a quality report event to one [QualityUi]. */
internal fun CoreEventView.QualityReport.toQualityUi(): QualityUi =
    QualityUi(
        toolCallId = toolCallId,
        info = counts.info.toIntClamped(),
        warn = counts.warn.toIntClamped(),
        block = counts.block.toIntClamped(),
        findings = findings.map { it.toQualityFindingUi() },
        skipped = skipped.toIntClamped(),
    )
