package app.tether.settings.budgets

import app.tether.core.generated.DayUsageView
import app.tether.core.generated.ModelUsageView
import app.tether.core.generated.ProjectUsageView
import app.tether.core.generated.ProviderView
import app.tether.core.generated.UsageSummaryView
import app.tether.core.generated.UsageTotalsView
import org.junit.Assert.assertEquals
import org.junit.Test

class CostMappingTest {
    private val todayTotals =
        UsageTotalsView(tokensIn = 91_563UL, tokensOut = 1_235UL, costMicros = 0UL)
    private val weekTotals =
        UsageTotalsView(tokensIn = 138_964UL, tokensOut = 4_732UL, costMicros = 0UL)

    private val view =
        UsageSummaryView(
            today = todayTotals,
            last7Days = weekTotals,
            last30Days = weekTotals,
            byDay =
                listOf(
                    DayUsageView(
                        day = "2026-11-28",
                        tokensIn = 91_563UL,
                        tokensOut = 1_235UL,
                        costMicros = 0UL,
                    ),
                ),
            byProject =
                listOf(
                    ProjectUsageView(
                        projectId = "p1",
                        projectName = "test project 1",
                        tokensIn = 138_964UL,
                        tokensOut = 4_732UL,
                        costMicros = 0UL,
                    ),
                ),
            byModel =
                listOf(
                    ModelUsageView(
                        provider = ProviderView.OPEN_ROUTER,
                        modelId = "nvidia/nemotron-3.5-super-vl:free",
                        tokensIn = 138_964UL,
                        tokensOut = 4_732UL,
                        costMicros = 0UL,
                    ),
                ),
        )

    @Test
    fun windowTotalsCarryTokensThrough() {
        val summary = view.toCostSummary()

        assertEquals(91_563L, summary.today.tokensIn)
        assertEquals(1_235L, summary.today.tokensOut)
        assertEquals(138_964L, summary.last7Days.tokensIn)
        assertEquals(4_732L, summary.last30Days.tokensOut)
    }

    @Test
    fun dayCarriesTokensAndTokenSum() {
        val day = view.toCostSummary().byDay[0]

        assertEquals("2026-11-28", day.day)
        assertEquals(91_563L, day.tokensIn)
        assertEquals(1_235L, day.tokensOut)
        assertEquals(92_798L, day.tokens)
    }

    @Test
    fun projectCarriesNameAndTokens() {
        val project = view.toCostSummary().byProject[0]

        assertEquals("p1", project.projectId)
        assertEquals("test project 1", project.name)
        assertEquals(138_964L, project.tokensIn)
        assertEquals(4_732L, project.tokensOut)
    }

    @Test
    fun modelCarriesProviderIdAndTokens() {
        val model = view.toCostSummary().byModel[0]

        assertEquals(ProviderView.OPEN_ROUTER, model.provider)
        assertEquals("nvidia/nemotron-3.5-super-vl:free", model.modelId)
        assertEquals(138_964L, model.tokensIn)
        assertEquals(4_732L, model.tokensOut)
    }

    @Test
    fun everyCostIsZero() {
        val summary = view.toCostSummary()

        assertEquals(0L, summary.today.costMicros)
        assertEquals(0L, summary.last7Days.costMicros)
        assertEquals(0L, summary.last30Days.costMicros)
        assertEquals(0L, summary.byDay[0].costMicros)
        assertEquals(0L, summary.byProject[0].costMicros)
        assertEquals(0L, summary.byModel[0].costMicros)
    }
}
