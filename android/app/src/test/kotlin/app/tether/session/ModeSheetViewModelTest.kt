package app.tether.session

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ModeSheetViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val fixture = SheetFixture()
    private var applied = 0
    private var biometric: BiometricOutcome = BiometricOutcome.Granted
    private var biometricAsked = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): ModeSheetViewModel {
        val vm =
            ModeSheetViewModel(
                deps = fixture.deps,
                authenticate = { onSuccess, onError ->
                    biometricAsked++
                    when (biometric) {
                        BiometricOutcome.Granted -> onSuccess()
                        BiometricOutcome.Denied -> onError()
                    }
                },
                onApplied = { applied++ },
                io = dispatcher,
            )
        advanceUntilIdle()
        return vm
    }

    @Test
    fun the_current_mode_is_read_from_the_session() =
        runTest(dispatcher) {
            assertEquals(SessionMode.ACCEPT_EDITS, model().state.value.current)
        }

    @Test
    fun a_plain_mode_is_set_and_the_runner_reopened_without_a_biometric_check() =
        runTest(dispatcher) {
            val vm = model()
            vm.choose(SessionMode.PLAN)
            advanceUntilIdle()
            assertEquals(SessionMode.PLAN, fixture.settings.session.mode)
            assertEquals(1, fixture.reopenCount)
            assertEquals(1, applied)
            assertEquals(0, biometricAsked)
        }

    @Test
    fun bypass_asks_for_confirmation_and_changes_nothing_yet() =
        runTest(dispatcher) {
            val vm = model()
            vm.choose(SessionMode.BYPASS)
            advanceUntilIdle()
            assertTrue(vm.state.value.confirming)
            assertTrue(fixture.settings.calls.none { it.startsWith("setMode") })
            assertEquals(0, biometricAsked)
        }

    @Test
    fun bypass_is_set_after_the_biometric_check_is_granted() =
        runTest(dispatcher) {
            biometric = BiometricOutcome.Granted
            val vm = model()
            vm.choose(SessionMode.BYPASS)
            vm.confirmBypass()
            advanceUntilIdle()
            assertEquals(SessionMode.BYPASS, fixture.settings.session.mode)
            assertEquals(1, fixture.reopenCount)
            assertEquals(1, applied)
            assertFalse(vm.state.value.confirming)
        }

    @Test
    fun a_denied_biometric_check_leaves_the_mode_alone() =
        runTest(dispatcher) {
            biometric = BiometricOutcome.Denied
            val vm = model()
            vm.choose(SessionMode.BYPASS)
            vm.confirmBypass()
            advanceUntilIdle()
            assertEquals(SessionMode.ACCEPT_EDITS, fixture.settings.session.mode)
            assertEquals(0, fixture.reopenCount)
            assertEquals(0, applied)
            assertTrue(vm.state.value.bypassDenied)
            assertTrue(vm.state.value.confirming)
        }

    @Test
    fun cancelling_the_confirmation_returns_to_the_list() =
        runTest(dispatcher) {
            val vm = model()
            vm.choose(SessionMode.BYPASS)
            vm.cancelBypass()
            assertFalse(vm.state.value.confirming)
            assertTrue(fixture.settings.calls.none { it.startsWith("setMode") })
        }

    @Test
    fun a_turn_in_progress_blocks_every_mode() =
        runTest(dispatcher) {
            val vm = model()
            fixture.running = true
            vm.choose(SessionMode.PLAN)
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("setMode") })
            assertEquals(0, applied)
        }
}

/** What the fake fingerprint or screen lock check reports. */
private enum class BiometricOutcome { Granted, Denied }
