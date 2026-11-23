package app.tether.session.cards

import app.tether.R
import app.tether.core.generated.PermissionModeView
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.PlanUi
import app.tether.session.agent.TodoStatus
import app.tether.session.agent.TodoUi

/** The longest answer the question card sends, in characters. */
const val ANSWER_MAX_CHARS = 500

/** The question draft shows a character counter from this length on. */
const val ANSWER_COUNTER_FROM = 400

/** How many checklist items are done, and how many there are. */
data class ChecklistCounts(
    val done: Int,
    val total: Int,
)

/** Counts the completed items of [items]. */
fun checklistCounts(items: List<TodoUi>): ChecklistCounts =
    ChecklistCounts(
        done = items.count { it.status == TodoStatus.Completed },
        total = items.size,
    )

/** True when the list has items and every one is completed. */
fun checklistAllDone(items: List<TodoUi>): Boolean =
    items.isNotEmpty() && items.all { it.status == TodoStatus.Completed }

/**
 * The item the collapsed pin shows: the first in-progress item, else the first pending item,
 * else the last item. Null for an empty list.
 */
fun currentChecklistItem(items: List<TodoUi>): TodoUi? =
    items.firstOrNull { it.status == TodoStatus.InProgress }
        ?: items.firstOrNull { it.status == TodoStatus.Pending }
        ?: items.lastOrNull()

/** The pin shows when there are items, unless the user dismissed it while all are done. */
fun checklistShown(
    items: List<TodoUi>,
    dismissed: Boolean,
): Boolean = items.isNotEmpty() && !(dismissed && checklistAllDone(items))

/** The answer to send for a draft: trimmed, then cut to [ANSWER_MAX_CHARS]. Null when blank. */
fun answerText(draft: String): String? {
    val trimmed = draft.trim()
    if (trimmed.isEmpty()) return null
    return trimmed.take(ANSWER_MAX_CHARS).trimEnd()
}

/**
 * Shortens [text] to at most [max] characters by cutting the middle and keeping both ends, with
 * an ellipsis in the cut. Text that already fits comes back unchanged.
 */
fun truncateMiddle(
    text: String,
    max: Int,
): String {
    if (text.length <= max) return text
    val keep = (max - 1).coerceAtLeast(2)
    val head = (keep + 1) / 2
    val tail = keep / 2
    return text.take(head) + ELLIPSIS + text.takeLast(tail)
}

private const val ELLIPSIS = "…"

/** The plan as editable text: the summary, then the steps numbered from 1. */
fun planAsText(plan: PlanUi): String =
    (listOf(plan.summary) + plan.steps.mapIndexed { index, step -> "${index + 1}. $step" })
        .joinToString(separator = "\n")

/** The modes a plan can run in. Plan and Bypass are never offered. Ask is the default. */
val RUN_PLAN_MODES: List<PermissionModeView> =
    listOf(
        PermissionModeView.ASK,
        PermissionModeView.ACCEPT_EDITS,
        PermissionModeView.AUTO,
    )

/** The message shown for a failed agent-loop call. */
fun AgentLoopFailure.messageRes(): Int =
    when (this) {
        AgentLoopFailure.NotFound -> R.string.cards_error_not_found
        AgentLoopFailure.Busy -> R.string.cards_error_busy
        AgentLoopFailure.Unavailable -> R.string.cards_error_unavailable
        AgentLoopFailure.Other -> R.string.cards_error_other
    }
