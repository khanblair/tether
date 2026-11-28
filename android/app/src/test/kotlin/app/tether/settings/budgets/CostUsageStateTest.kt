package app.tether.settings.budgets

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CostUsageStateTest {
    @Test
    fun noSummaryIsNeitherEmptyNorFreeOnly() {
        val state = CostUiState(summary = null)

        assertFalse(state.empty)
        assertFalse(state.freeOnly)
    }

    @Test
    fun zeroCostAndZeroTokenDaysAreEmpty() {
        val state = CostUiState(summary = summaryWith(daysOf(0L, 0L)))

        assertTrue(state.empty)
        assertFalse(state.freeOnly)
        assertEquals(0L, state.spentInRange)
        assertEquals(0L, state.tokensInRange)
    }

    @Test
    fun freeModelTokensWithoutSpendAreFreeOnly() {
        val days =
            listOf(
                CostDay("2026-11-28", 0, 91_563, 1_235),
                CostDay("2026-11-27", 0),
            )
        val state = CostUiState(summary = summaryWith(days))

        assertFalse(state.empty)
        assertTrue(state.freeOnly)
        assertEquals(0L, state.spentInRange)
        assertEquals(92_798L, state.tokensInRange)
    }

    @Test
    fun daysWithSpendAreNeitherEmptyNorFreeOnly() {
        val days = listOf(CostDay("2026-11-28", 250_000, 91_563, 1_235))
        val state = CostUiState(summary = summaryWith(days))

        assertFalse(state.empty)
        assertFalse(state.freeOnly)
        assertEquals(250_000L, state.spentInRange)
    }

    @Test
    fun dayWithOnlySpendIsNeitherEmptyNorFreeOnly() {
        val days = listOf(CostDay("2026-11-28", 250_000))
        val state = CostUiState(summary = summaryWith(days))

        assertFalse(state.empty)
        assertFalse(state.freeOnly)
        assertEquals(0L, state.tokensInRange)
    }

    @Test
    fun costDayTokensAreInPlusOut() {
        val day = CostDay("2026-11-28", 0, 91_563, 1_235)

        assertEquals(92_798L, day.tokens)
    }
}
