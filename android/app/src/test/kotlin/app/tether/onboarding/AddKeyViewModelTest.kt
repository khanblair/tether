package app.tether.onboarding

import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView
import app.tether.providers.FakeProviderGateway
import app.tether.providers.ProviderGateway
import app.tether.providers.ProviderMessage
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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class AddKeyViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProviderGateway()
    private var accepted = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): AddKeyViewModel {
        val vm = AddKeyViewModel(gateway, onAccepted = { accepted++ }, io = dispatcher)
        advanceUntilIdle()
        return vm
    }

    private fun TestScope.typedAndSaved(status: ConnectionStatusView): AddKeyViewModel {
        gateway.testResult = ProviderStatusView(status, 0u, null)
        val vm = model()
        vm.onKeyChange("sk-test")
        vm.saveAndTest()
        advanceUntilIdle()
        return vm
    }

    @Test
    fun the_step_starts_idle_on_the_first_provider_with_an_empty_field() =
        runTest(dispatcher) {
            val state = model().state.value

            assertEquals(ProviderView.ANTHROPIC, state.provider)
            assertEquals(AddKeyPhase.Idle, state.phase)
            assertEquals("", state.keyInput)
            assertFalse(state.canSave)
        }

    @Test
    fun a_blank_key_cannot_be_saved() =
        runTest(dispatcher) {
            val vm = model()

            vm.onKeyChange("   ")
            vm.saveAndTest()
            advanceUntilIdle()

            assertFalse(vm.state.value.canSave)
            assertTrue(gateway.calls.isEmpty())
        }

    @Test
    fun an_accepted_key_saves_tests_clears_the_field_and_moves_on_once() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.OK)

            assertEquals(listOf("setKey", "testConnection"), gateway.calls)
            assertEquals("sk-test", gateway.keys[ProviderView.ANTHROPIC])
            assertEquals(AddKeyPhase.Tested, vm.state.value.phase)
            assertEquals(ConnectionStatusView.OK, vm.state.value.result)
            assertTrue(vm.state.value.accepted)
            assertEquals("", vm.state.value.keyInput)
            assertEquals(1, accepted)
        }

    @Test
    fun a_rejected_key_keeps_the_result_and_does_not_move_on() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.BAD_KEY)

            assertEquals(AddKeyPhase.Tested, vm.state.value.phase)
            assertEquals(ConnectionStatusView.BAD_KEY, vm.state.value.result)
            assertFalse(vm.state.value.accepted)
            assertEquals(0, accepted)
        }

    @Test
    fun a_network_error_is_kept_as_the_result() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.NETWORK_ERROR)

            assertEquals(ConnectionStatusView.NETWORK_ERROR, vm.state.value.result)
            assertEquals(0, accepted)
        }

    @Test
    fun a_block_by_region_is_kept_as_the_result() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.BLOCKED)

            assertEquals(ConnectionStatusView.BLOCKED, vm.state.value.result)
            assertEquals(0, accepted)
        }

    @Test
    fun a_status_with_no_message_is_a_failed_test() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.UNSET)

            assertEquals(AddKeyPhase.Failed, vm.state.value.phase)
            assertEquals(ProviderMessage.TestFailed, vm.state.value.message)
            assertEquals(0, accepted)
        }

    @Test
    fun a_save_that_throws_keeps_the_typed_key_for_another_try() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")
            gateway.failNext = IllegalStateException("disk full")

            vm.saveAndTest()
            advanceUntilIdle()

            assertEquals(AddKeyPhase.Failed, vm.state.value.phase)
            assertEquals(ProviderMessage.SaveFailed, vm.state.value.message)
            assertEquals("sk-test", vm.state.value.keyInput)
            assertEquals(listOf("setKey"), gateway.calls)
        }

    @Test
    fun editing_the_key_clears_the_last_result() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.BAD_KEY)

            vm.onKeyChange("sk-new")

            assertEquals(AddKeyPhase.Idle, vm.state.value.phase)
            assertEquals(ConnectionStatusView.UNSET, vm.state.value.result)
            assertNull(vm.state.value.message)
        }

    @Test
    fun choosing_a_provider_clears_the_last_result() =
        runTest(dispatcher) {
            val vm = typedAndSaved(ConnectionStatusView.BAD_KEY)

            vm.selectProvider(ProviderView.OPEN_AI)

            assertEquals(ProviderView.OPEN_AI, vm.state.value.provider)
            assertEquals(AddKeyPhase.Idle, vm.state.value.phase)
        }

    @Test
    fun the_phase_is_testing_while_the_provider_is_called() =
        runTest(dispatcher) {
            lateinit var vm: AddKeyViewModel
            var seen: AddKeyPhase? = null
            val spy =
                object : ProviderGateway by gateway {
                    override fun testConnection(provider: ProviderView): ProviderStatusView {
                        seen = vm.state.value.phase
                        return gateway.testConnection(provider)
                    }
                }
            vm = AddKeyViewModel(spy, onAccepted = {}, io = dispatcher)
            advanceUntilIdle()
            vm.onKeyChange("sk-test")

            vm.saveAndTest()
            advanceUntilIdle()

            assertEquals(AddKeyPhase.Testing, seen)
            assertEquals(AddKeyPhase.Tested, vm.state.value.phase)
        }

    @Test
    fun stopping_the_screen_masks_the_revealed_text_and_keeps_it() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")
            vm.toggleReveal()

            vm.onStopped()

            assertFalse(vm.state.value.revealed)
            assertEquals("sk-test", vm.state.value.keyInput)
        }

    @Test
    fun the_reveal_toggle_only_changes_what_is_shown() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")

            vm.toggleReveal()
            assertTrue(vm.state.value.revealed)
            vm.toggleReveal()
            assertFalse(vm.state.value.revealed)
            assertEquals("sk-test", vm.state.value.keyInput)
        }

    @Test
    fun the_state_text_leaves_out_the_key() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-secret-value")

            assertFalse(
                vm.state.value
                    .toString()
                    .contains("sk-secret-value"),
            )
        }
}
