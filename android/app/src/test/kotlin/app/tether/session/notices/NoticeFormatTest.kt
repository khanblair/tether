package app.tether.session.notices

import app.tether.R
import app.tether.session.agent.BudgetScope
import org.junit.Assert.assertEquals
import org.junit.Test

class NoticeFormatTest {
    @Test
    fun contextBandEdgesBelongToTheLowerBand() {
        assertEquals(ContextBand.Neutral, contextBand(0))
        assertEquals(ContextBand.Neutral, contextBand(59))
        assertEquals(ContextBand.Warning, contextBand(60))
        assertEquals(ContextBand.Warning, contextBand(79))
        assertEquals(ContextBand.Warning, contextBand(80))
        assertEquals(ContextBand.Danger, contextBand(81))
        assertEquals(ContextBand.Danger, contextBand(100))
    }

    @Test
    fun costAmountFormatsMicrosAsDollars() {
        assertEquals(CostAmount.Dollars("\$0.00"), costAmount(0L))
        assertEquals(CostAmount.UnderCent, costAmount(1L))
        assertEquals(CostAmount.UnderCent, costAmount(9_999L))
        assertEquals(CostAmount.Dollars("\$0.01"), costAmount(10_000L))
        assertEquals(CostAmount.Dollars("\$0.42"), costAmount(420_000L))
        assertEquals(CostAmount.Dollars("\$2.00"), costAmount(2_000_000L))
    }

    @Test
    fun dollarsTextAlwaysShowsTwoDecimals() {
        assertEquals("\$2.00", dollarsText(2_000_000L))
        assertEquals("\$0.42", dollarsText(420_000L))
    }

    @Test
    fun budgetStateMarksTheEightyPercentAndTheLimit() {
        assertEquals(BudgetState.NoBudget, budgetState(500_000L, null))
        assertEquals(BudgetState.NoBudget, budgetState(500_000L, 0L))
        assertEquals(BudgetState.Under, budgetState(1_599_999L, 2_000_000L))
        assertEquals(BudgetState.Near, budgetState(1_600_000L, 2_000_000L))
        assertEquals(BudgetState.Near, budgetState(1_999_999L, 2_000_000L))
        assertEquals(BudgetState.Reached, budgetState(2_000_000L, 2_000_000L))
    }

    @Test
    fun budgetWarningTextNamesTheScope() {
        assertEquals(R.string.notices_budget_warning_session, budgetWarningRes(BudgetScope.Session))
        assertEquals(R.string.notices_budget_warning_day, budgetWarningRes(BudgetScope.Day))
    }

    @Test
    fun budgetReachedTextNamesTheScope() {
        assertEquals(R.string.notices_budget_reached_session, budgetReachedRes(BudgetScope.Session))
        assertEquals(R.string.notices_budget_reached_day, budgetReachedRes(BudgetScope.Day))
    }
}
