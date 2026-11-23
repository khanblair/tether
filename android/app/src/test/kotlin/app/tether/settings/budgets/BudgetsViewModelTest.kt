package app.tether.settings.budgets

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class BudgetsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(gateway: FakeBudgetsGateway): BudgetsViewModel =
        BudgetsViewModel(gateway, dispatcher).also { dispatcher.scheduler.advanceUntilIdle() }

    @Test
    fun saved_limits_fill_the_fields_and_nothing_is_savable_yet() {
        val gateway =
            FakeBudgetsGateway(BudgetLimits(dailyMicros = 2_500_000L, sessionMicros = null))

        val state = viewModel(gateway).state.value

        assertFalse(state.loading)
        assertEquals("2.50", state.daily)
        assertEquals("", state.session)
        assertFalse(state.canSave)
    }

    @Test
    fun a_failed_load_shows_the_failure_and_retry_loads_again() {
        val gateway =
            FakeBudgetsGateway(BudgetLimits(dailyMicros = 1_000_000L, sessionMicros = null))
        gateway.loadFailure = BudgetsFailure(BudgetsProblem.Unavailable)
        val vm = viewModel(gateway)
        assertTrue(vm.state.value.loadFailed)
        assertFalse(vm.state.value.canSave)

        gateway.loadFailure = null
        vm.retry()
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.state.value.loadFailed)
        assertEquals("1.00", vm.state.value.daily)
    }

    @Test
    fun a_too_precise_daily_limit_is_marked_and_blocks_save() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)

        vm.onDailyChange("2.505")

        assertEquals(
            DollarInput.Invalid(DollarProblem.TooManyDecimals),
            vm.state.value.dailyInput,
        )
        assertFalse(vm.state.value.canSave)
    }

    @Test
    fun a_negative_session_limit_is_marked_and_blocks_save() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)

        vm.onSessionChange("-1")

        assertEquals(DollarInput.Invalid(DollarProblem.Negative), vm.state.value.sessionInput)
        assertFalse(vm.state.value.valid)
        assertFalse(vm.state.value.canSave)
    }

    @Test
    fun letters_in_the_daily_limit_are_not_a_number() {
        val vm = viewModel(FakeBudgetsGateway())

        vm.onDailyChange("abc")

        assertEquals(DollarInput.Invalid(DollarProblem.NotANumber), vm.state.value.dailyInput)
    }

    @Test
    fun save_calls_set_budgets_once_with_both_parsed_values_in_order() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)
        vm.onDailyChange("2,50")
        vm.onSessionChange("0.75")

        vm.save()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf(2_500_000L to 750_000L), gateway.saves)
        assertTrue(vm.state.value.saved)
        assertFalse(vm.state.value.saving)
    }

    @Test
    fun a_second_save_while_saving_does_not_call_the_core_again() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)
        vm.onDailyChange("2")

        vm.save()
        vm.save()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, gateway.saves.size)
    }

    @Test
    fun after_a_save_the_form_is_unchanged_until_it_is_edited_again() {
        val gateway = FakeBudgetsGateway()
        val vm = viewModel(gateway)
        vm.onDailyChange("2")
        vm.save()
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(vm.state.value.canSave)

        vm.onDailyChange("3")

        assertFalse(vm.state.value.saved)
        assertTrue(vm.state.value.canSave)
    }

    @Test
    fun remove_limit_saves_null_for_that_limit() {
        val gateway =
            FakeBudgetsGateway(BudgetLimits(dailyMicros = 2_000_000L, sessionMicros = 500_000L))
        val vm = viewModel(gateway)

        vm.removeDaily()
        vm.save()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf(null to 500_000L), gateway.saves)
        assertEquals("", vm.state.value.daily)
    }

    @Test
    fun a_failed_save_keeps_the_typed_values_and_shows_the_problem() {
        val gateway = FakeBudgetsGateway()
        gateway.saveFailure = BudgetsFailure(BudgetsProblem.Unavailable)
        val vm = viewModel(gateway)
        vm.onDailyChange("3")

        vm.save()
        dispatcher.scheduler.advanceUntilIdle()

        val state = vm.state.value
        assertEquals("3", state.daily)
        assertEquals(BudgetsProblem.Unavailable, state.saveFailure)
        assertFalse(state.saved)
        assertFalse(state.saving)
        assertTrue(state.canSave)
    }

    @Test
    fun a_limit_the_core_rejects_shows_the_range_message() {
        val gateway = FakeBudgetsGateway()
        gateway.saveFailure = BudgetsFailure(BudgetsProblem.InvalidBudget)
        val vm = viewModel(gateway)
        vm.onDailyChange("1")

        vm.save()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(BudgetsProblem.InvalidBudget, vm.state.value.saveFailure)
    }

    @Test
    fun a_save_failure_clears_when_the_form_is_edited() {
        val gateway = FakeBudgetsGateway()
        gateway.saveFailure = BudgetsFailure(BudgetsProblem.Unavailable)
        val vm = viewModel(gateway)
        vm.onDailyChange("1")
        vm.save()
        dispatcher.scheduler.advanceUntilIdle()

        vm.onDailyChange("1.5")

        assertEquals(null, vm.state.value.saveFailure)
    }
}
