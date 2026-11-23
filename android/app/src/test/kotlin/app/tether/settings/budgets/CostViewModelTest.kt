package app.tether.settings.budgets

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CostViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(gateway: FakeBudgetsGateway): CostViewModel =
        CostViewModel(gateway, { OFFSET_MINUTES }, dispatcher).also {
            dispatcher.scheduler.advanceUntilIdle()
        }

    @Test
    fun the_first_load_asks_for_seven_days_at_the_device_offset() {
        val gateway = FakeBudgetsGateway(summary = summaryWith(daysOf(0L, 420_000L)))

        val vm = viewModel(gateway)

        assertEquals(listOf(7 to OFFSET_MINUTES), gateway.summaryCalls)
        assertEquals(CostRange.Week, vm.state.value.range)
        assertNotNull(vm.state.value.summary)
        assertFalse(vm.state.value.loading)
    }

    @Test
    fun switching_to_thirty_days_reloads_with_thirty_days() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)

        vm.select(CostRange.Month)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(
            listOf(7 to OFFSET_MINUTES, 30 to OFFSET_MINUTES),
            gateway.summaryCalls,
        )
        assertEquals(CostRange.Month, vm.state.value.range)
    }

    @Test
    fun selecting_the_range_already_shown_does_not_reload() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)

        vm.select(CostRange.Week)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, gateway.summaryCalls.size)
    }

    @Test
    fun a_range_with_no_spend_is_the_empty_state() {
        val gateway = FakeBudgetsGateway(summary = summaryWith(daysOf(0L, 0L, 0L)))

        val vm = viewModel(gateway)

        assertTrue(vm.state.value.empty)
        assertEquals(0L, vm.state.value.spentInRange)
    }

    @Test
    fun a_range_with_spend_is_not_empty() {
        val gateway = FakeBudgetsGateway(summary = summaryWith(daysOf(0L, 420_000L)))

        val vm = viewModel(gateway)

        assertFalse(vm.state.value.empty)
        assertEquals(420_000L, vm.state.value.spentInRange)
    }

    @Test
    fun nothing_is_empty_before_the_summary_loads() {
        val gateway = FakeBudgetsGateway()
        gateway.summaryFailure = BudgetsFailure(BudgetsProblem.Unavailable)

        val vm = viewModel(gateway)

        assertFalse(vm.state.value.empty)
        assertTrue(vm.state.value.failed)
    }

    @Test
    fun a_failed_load_shows_the_failure_and_retry_recovers() {
        val gateway = FakeBudgetsGateway()
        gateway.summaryFailure = BudgetsFailure(BudgetsProblem.Unavailable)
        val vm = viewModel(gateway)
        assertTrue(vm.state.value.failed)
        assertEquals(null, vm.state.value.summary)

        gateway.summaryFailure = null
        vm.retry()
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.state.value.failed)
        assertNotNull(vm.state.value.summary)
    }

    @Test
    fun the_totals_come_through_unchanged() {
        val gateway =
            FakeBudgetsGateway(
                summary =
                    summaryWith(daysOf(0L)).copy(
                        today = totalsOf(840_000L),
                        last7Days = totalsOf(1_500_000L),
                        last30Days = totalsOf(4_000_000L),
                    ),
            )

        val summary = viewModel(gateway).state.value.summary

        assertEquals(840_000L, summary?.today?.costMicros)
        assertEquals(1_500_000L, summary?.last7Days?.costMicros)
        assertEquals(4_000_000L, summary?.last30Days?.costMicros)
    }

    private companion object {
        const val OFFSET_MINUTES = 180
    }
}
