package app.tether.session.agent

import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import app.tether.session.approval.ApprovalScreenState
import app.tether.session.cards.CardsViewState
import app.tether.session.notices.NoticesState
import app.tether.session.tools.CallRow
import app.tether.session.tools.groupCalls

/**
 * What the session screen draws from the agent loop, read at one moment. [loop] gives the
 * ViewModels' actions. [budgetMicros] is the limit the cost meter shows, when a budget notice has
 * named one.
 */
data class AgentScreenUi(
    val loop: AgentLoop,
    val state: AgentLoopState,
    val approvals: ApprovalScreenState,
    val cards: CardsViewState,
    val notices: NoticesState,
    val budgetMicros: Long?,
) {
    /** True while an approval waits. The user cannot send a message until it is answered. */
    val approvalsWaiting: Boolean
        get() = approvals.approvals.isNotEmpty()
}

/** Collects the agent-loop flows of [loop] for the composition. Null when [loop] is null. */
@Composable
fun rememberAgentScreen(loop: AgentLoop?): AgentScreenUi? = loop?.let { collectAgentScreen(it) }

@Composable
private fun collectAgentScreen(loop: AgentLoop): AgentScreenUi {
    val state by loop.state.collectAsState()
    val approvals by loop.approvals.state.collectAsState()
    val cards by loop.cards.state.collectAsState()
    val notices by loop.notices.state.collectAsState()
    return AgentScreenUi(
        loop = loop,
        state = state,
        approvals = approvals,
        cards = cards,
        notices = notices,
        budgetMicros = budgetLimitMicros(state.banners),
    )
}

/**
 * One item of the live section after the last message: a tool card or group, a quality report, the
 * question card, or the plan card. [key] keeps the lazy list's item identity stable.
 */
sealed interface LiveItem {
    /** The key of this item in the lazy list. */
    val key: String

    /** A tool call, or a group of read-only calls. */
    data class Tools(
        val row: CallRow,
    ) : LiveItem {
        override val key: String
            get() =
                when (row) {
                    is CallRow.Single -> "call:${row.call.id}"
                    is CallRow.Group -> "group:${row.calls.first().id}"
                }
    }

    /** A quality report with findings. [index] is its place in the report list. */
    data class Report(
        val report: QualityUi,
        val index: Int,
    ) : LiveItem {
        override val key: String
            get() = "quality:$index"
    }

    /** The question card. Its content comes from the cards state. */
    data object Ask : LiveItem {
        override val key: String = "ask"
    }

    /** The plan card. Its content comes from the cards state. */
    data object Plan : LiveItem {
        override val key: String = "plan"
    }
}

/**
 * The live section of the current turn, in order: the tool rows, then the quality report of the
 * last call, then the question, then the plan. A report with no findings is left out.
 */
fun liveItems(
    calls: List<ToolCallUi>,
    quality: List<QualityUi>,
    question: QuestionUi?,
    plan: PlanUi?,
): List<LiveItem> {
    val items = mutableListOf<LiveItem>()
    groupCalls(calls).mapTo(items) { LiveItem.Tools(it) }
    val lastId = calls.lastOrNull()?.id
    quality.forEachIndexed { index, report ->
        if (report.toolCallId == lastId && report.findings.isNotEmpty()) {
            items += LiveItem.Report(report, index)
        }
    }
    if (question != null) items += LiveItem.Ask
    if (plan != null) items += LiveItem.Plan
    return items
}

/** The live section of [ui]. Empty when there is nothing live. */
fun AgentScreenUi.liveSection(): List<LiveItem> =
    liveItems(state.calls, state.quality, cards.question, cards.plan)

/**
 * The budget limit the cost meter compares against. It prefers the session budget, then the day
 * budget. Null when no budget notice has named a limit yet.
 */
fun budgetLimitMicros(banners: List<AgentBanner>): Long? {
    val limits =
        banners.mapNotNull { banner ->
            when (banner) {
                is AgentBanner.BudgetWarning -> banner.scope to banner.limitMicros
                is AgentBanner.BudgetReached -> banner.scope to banner.limitMicros
                else -> null
            }
        }
    return (limits.firstOrNull { it.first == BudgetScope.Session } ?: limits.firstOrNull())?.second
}
