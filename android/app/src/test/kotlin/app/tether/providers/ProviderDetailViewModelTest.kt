package app.tether.providers

import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderSettingsException
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView
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
class ProviderDetailViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProviderGateway()
    private val provider = ProviderView.ANTHROPIC

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): ProviderDetailViewModel {
        val vm = ProviderDetailViewModel(provider, gateway, dispatcher)
        advanceUntilIdle()
        return vm
    }

    @Test
    fun load_reads_the_saved_key_status_and_target() =
        runTest(dispatcher) {
            gateway.keys[provider] = "k"
            gateway.statuses[provider] = ProviderStatusView(ConnectionStatusView.OK, 14u, 5_000L)
            val state = model().state.value
            assertFalse(state.loading)
            assertTrue(state.hasKey)
            assertEquals(ConnectionStatusView.OK, state.status)
            assertEquals(14, state.modelCount)
            assertEquals(5_000L, state.testedAtMs)
            assertEquals("https://example.test/anthropic/models", state.testTarget)
            assertNull(state.message)
        }

    @Test
    fun a_provider_with_no_key_reads_unset_even_when_a_status_is_saved() =
        runTest(dispatcher) {
            gateway.statuses[provider] = ProviderStatusView(ConnectionStatusView.OK, 2u, 9L)
            val state = model().state.value
            assertFalse(state.hasKey)
            assertEquals(ConnectionStatusView.UNSET, state.status)
        }

    @Test
    fun a_failed_load_shows_a_message_and_stops_loading() =
        runTest(dispatcher) {
            gateway.failNext = IllegalStateException("boom")
            val state = model().state.value
            assertFalse(state.loading)
            assertEquals(ProviderMessage.LoadFailed, state.message)
        }

    @Test
    fun typing_marks_the_key_edited_and_enables_save_only_for_text() =
        runTest(dispatcher) {
            val vm = model()
            assertFalse(vm.state.value.canSave)
            vm.onKeyChange("   ")
            assertTrue(vm.state.value.edited)
            assertFalse(vm.state.value.canSave)
            vm.onKeyChange("sk-test")
            assertTrue(vm.state.value.canSave)
        }

    @Test
    fun saving_sends_the_key_then_clears_the_field_and_resets_the_status() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")
            vm.save()
            assertTrue(vm.state.value.saving)
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals("sk-test", gateway.keys[provider])
            assertEquals("", state.keyInput)
            assertFalse(state.edited)
            assertFalse(state.saving)
            assertTrue(state.hasKey)
            assertEquals(ConnectionStatusView.UNSET, state.status)
        }

    @Test
    fun a_second_save_while_saving_is_ignored() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")
            vm.save()
            vm.save()
            advanceUntilIdle()
            assertEquals(1, gateway.calls.count { it == "setKey" })
        }

    @Test
    fun an_edit_made_while_a_save_is_pending_is_ignored() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("first")
            vm.save()
            assertTrue(vm.state.value.saving)
            vm.onKeyChange("second")
            assertEquals("first", vm.state.value.keyInput)
            advanceUntilIdle()
            assertEquals("first", gateway.keys[provider])
            assertEquals("", vm.state.value.keyInput)
            vm.onKeyChange("second")
            assertEquals("second", vm.state.value.keyInput)
        }

    @Test
    fun an_empty_key_error_keeps_the_text_and_shows_its_own_message() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")
            gateway.failNext = ProviderSettingsException.EmptyKey()
            vm.save()
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals(ProviderMessage.EmptyKey, state.message)
            assertEquals("sk-test", state.keyInput)
            assertFalse(state.saving)
        }

    @Test
    fun any_other_save_error_shows_the_generic_save_message() =
        runTest(dispatcher) {
            val vm = model()
            vm.onKeyChange("sk-test")
            gateway.failNext = ProviderSettingsException.Unavailable()
            vm.save()
            advanceUntilIdle()
            assertEquals(ProviderMessage.SaveFailed, vm.state.value.message)
            assertFalse(vm.state.value.saving)
        }

    @Test
    fun a_test_shows_testing_then_the_saved_result() =
        runTest(dispatcher) {
            gateway.keys[provider] = "k"
            val vm = model()
            vm.test()
            assertTrue(vm.state.value.testing)
            advanceUntilIdle()
            val state = vm.state.value
            assertFalse(state.testing)
            assertEquals(ConnectionStatusView.OK, state.status)
            assertEquals(3, state.modelCount)
            assertEquals(1_000L, state.testedAtMs)
        }

    @Test
    fun every_failure_status_from_the_core_is_kept_as_it_came() =
        runTest(dispatcher) {
            gateway.keys[provider] = "k"
            val vm = model()
            for (status in listOf(
                ConnectionStatusView.BAD_KEY,
                ConnectionStatusView.NETWORK_ERROR,
                ConnectionStatusView.BLOCKED,
            )) {
                gateway.testResult = ProviderStatusView(status, 0u, 2_000L)
                vm.test()
                advanceUntilIdle()
                assertEquals(status, vm.state.value.status)
            }
        }

    @Test
    fun a_test_that_throws_keeps_the_old_status_and_shows_a_message() =
        runTest(dispatcher) {
            gateway.keys[provider] = "k"
            gateway.statuses[provider] = ProviderStatusView(ConnectionStatusView.OK, 4u, 7L)
            val vm = model()
            gateway.failNext = ProviderSettingsException.Unavailable()
            vm.test()
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals(ProviderMessage.TestFailed, state.message)
            assertEquals(ConnectionStatusView.OK, state.status)
            assertFalse(state.testing)
        }

    @Test
    fun a_test_is_refused_without_a_key_or_with_an_unsaved_edit() =
        runTest(dispatcher) {
            val vm = model()
            vm.test()
            advanceUntilIdle()
            assertFalse("testConnection" in gateway.calls)
            gateway.keys[provider] = "k"
            vm.load()
            advanceUntilIdle()
            vm.onKeyChange("new")
            vm.test()
            advanceUntilIdle()
            assertFalse("testConnection" in gateway.calls)
        }

    @Test
    fun removing_a_key_resets_everything() =
        runTest(dispatcher) {
            gateway.keys[provider] = "k"
            gateway.statuses[provider] = ProviderStatusView(ConnectionStatusView.OK, 4u, 7L)
            val vm = model()
            vm.remove()
            advanceUntilIdle()
            val state = vm.state.value
            assertFalse(state.hasKey)
            assertEquals(ConnectionStatusView.UNSET, state.status)
            assertEquals(0, state.modelCount)
            assertNull(state.testedAtMs)
            assertFalse("k" in gateway.keys.values)
        }

    @Test
    fun a_failed_remove_keeps_the_key_and_shows_a_message() =
        runTest(dispatcher) {
            gateway.keys[provider] = "k"
            val vm = model()
            gateway.failNext = ProviderSettingsException.Unavailable()
            vm.remove()
            advanceUntilIdle()
            assertTrue(vm.state.value.hasKey)
            assertEquals(ProviderMessage.RemoveFailed, vm.state.value.message)
            assertFalse(vm.state.value.saving)
        }

    @Test
    fun reveal_shows_the_saved_key_and_hide_drops_it() =
        runTest(dispatcher) {
            gateway.keys[provider] = "sk-saved"
            val vm = model()
            vm.reveal()
            advanceUntilIdle()
            assertEquals("sk-saved", vm.state.value.keyInput)
            assertTrue(vm.state.value.revealed)
            assertFalse(vm.state.value.edited)
            vm.hide()
            assertEquals("", vm.state.value.keyInput)
            assertFalse(vm.state.value.revealed)
        }

    @Test
    fun reveal_with_no_key_or_an_error_shows_a_message() =
        runTest(dispatcher) {
            val vm = model()
            vm.reveal()
            advanceUntilIdle()
            assertEquals(ProviderMessage.RevealFailed, vm.state.value.message)
            gateway.keys[provider] = "k"
            vm.dismissMessage()
            gateway.failNext = ProviderSettingsException.Unavailable()
            vm.reveal()
            advanceUntilIdle()
            assertEquals(ProviderMessage.RevealFailed, vm.state.value.message)
            assertFalse(vm.state.value.revealed)
        }

    @Test
    fun stopping_drops_a_revealed_saved_key_but_only_masks_typed_text() =
        runTest(dispatcher) {
            gateway.keys[provider] = "sk-saved"
            val vm = model()
            vm.reveal()
            advanceUntilIdle()
            vm.onStopped()
            assertEquals("", vm.state.value.keyInput)
            assertFalse(vm.state.value.revealed)
            vm.onKeyChange("typed")
            vm.show()
            vm.onStopped()
            assertEquals("typed", vm.state.value.keyInput)
            assertFalse(vm.state.value.revealed)
        }

    @Test
    fun the_eye_button_follows_what_the_field_holds() =
        runTest(dispatcher) {
            val vm = model()
            assertEquals(RevealAction.None, vm.state.value.revealAction)
            vm.onKeyChange("typed")
            assertEquals(RevealAction.Show, vm.state.value.revealAction)
            vm.show()
            assertEquals(RevealAction.Hide, vm.state.value.revealAction)
            vm.hide()
            assertEquals("typed", vm.state.value.keyInput)
            assertEquals(RevealAction.Show, vm.state.value.revealAction)
            gateway.keys[provider] = "sk-saved"
            vm.save()
            advanceUntilIdle()
            assertEquals(RevealAction.Authenticate, vm.state.value.revealAction)
            vm.reveal()
            advanceUntilIdle()
            assertEquals(RevealAction.Hide, vm.state.value.revealAction)
        }

    @Test
    fun show_does_nothing_for_a_field_the_user_has_not_typed_in() =
        runTest(dispatcher) {
            gateway.keys[provider] = "sk-saved"
            val vm = model()
            vm.show()
            assertFalse(vm.state.value.revealed)
            assertEquals("", vm.state.value.keyInput)
        }

    @Test
    fun the_state_text_never_contains_the_key() =
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
