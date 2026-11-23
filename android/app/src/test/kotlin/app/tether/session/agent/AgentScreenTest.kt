package app.tether.session.agent

import app.tether.session.tools.CallRow
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AgentScreenTest {
    private fun call(
        id: String,
        tool: String = "read_file",
        status: ToolStatus = ToolStatus.Succeeded,
    ) = ToolCallUi(id, tool, "", status, "", false)

    private fun report(
        toolCallId: String,
        findings: Int,
    ) = QualityUi(
        toolCallId = toolCallId,
        info = 0,
        warn = findings,
        block = 0,
        findings =
            List(findings) {
                QualityFindingUi("r$it", FindingSeverity.Warn, "a.kt", 1, "m", "a")
            },
        skipped = 0,
    )

    private val question = QuestionUi("q1", "Which?", listOf("A"), false)
    private val plan = PlanUi(null, "Plan", emptyList(), emptyList(), emptyList(), emptyList())

    @Test
    fun readOnlyCallsGroupAndRunCallsStandAlone() {
        val items =
            liveItems(
                calls = listOf(call("t1"), call("t2"), call("t3", tool = "run")),
                quality = emptyList(),
                question = null,
                plan = null,
            )

        assertEquals(
            listOf("group:t1", "call:t3"),
            items.map { it.key },
        )
        val group = (items.first() as LiveItem.Tools).row as CallRow.Group
        assertEquals(2, group.calls.size)
    }

    @Test
    fun aQualityReportShowsOnlyForTheLastCallAndOnlyWithFindings() {
        val items =
            liveItems(
                calls = listOf(call("t1", tool = "run"), call("t2", tool = "run")),
                quality =
                    listOf(
                        report("t1", findings = 1),
                        report("t2", findings = 0),
                        report("t2", findings = 2),
                    ),
                question = null,
                plan = null,
            )

        assertEquals(listOf("call:t1", "call:t2", "quality:2"), items.map { it.key })
    }

    @Test
    fun aReportForAnEarlierCallIsLeftOut() {
        val items =
            liveItems(
                calls = listOf(call("t1", tool = "run"), call("t2", tool = "run")),
                quality = listOf(report("t1", findings = 1)),
                question = null,
                plan = null,
            )

        assertEquals(listOf("call:t1", "call:t2"), items.map { it.key })
    }

    @Test
    fun theQuestionAndThePlanFollowTheCalls() {
        val items =
            liveItems(
                calls = listOf(call("t1", tool = "run")),
                quality = emptyList(),
                question = question,
                plan = plan,
            )

        assertEquals(listOf("call:t1", "ask", "plan"), items.map { it.key })
    }

    @Test
    fun nothingLiveGivesAnEmptySection() {
        assertEquals(emptyList<LiveItem>(), liveItems(emptyList(), emptyList(), null, null))
    }

    @Test
    fun theSessionBudgetLimitWins() {
        val banners =
            listOf(
                AgentBanner.BudgetWarning(BudgetScope.Day, 80, 800L, 1_000L),
                AgentBanner.BudgetWarning(BudgetScope.Session, 90, 900L, 1_000L),
            )

        assertEquals(1_000L, budgetLimitMicros(banners))
    }

    @Test
    fun theDayLimitIsUsedWhenNoSessionLimitIsKnown() {
        val banners = listOf(AgentBanner.BudgetReached(BudgetScope.Day, 2_000L, 2_000L))

        assertEquals(2_000L, budgetLimitMicros(banners))
    }

    @Test
    fun noBudgetNoticeMeansNoLimit() {
        assertNull(budgetLimitMicros(listOf(AgentBanner.NetworkPaused)))
    }

    @Test
    fun anExpandedCardToggles() {
        val cards = ExpandedCards()

        cards.toggle("t1")
        assertEquals(setOf("t1"), cards.ids)

        cards.toggle("t1")
        assertEquals(emptySet<String>(), cards.ids)
    }
}
