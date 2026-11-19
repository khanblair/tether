package app.tether.providers

import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
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
class ProvidersViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProviderGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun the_list_starts_with_all_five_providers_loading() =
        runTest(dispatcher) {
            val state = ProvidersViewModel(gateway, dispatcher).state.value
            assertTrue(state.loading)
            assertEquals(ProviderView.entries, state.items.map { it.provider })
        }

    @Test
    fun refresh_shows_each_provider_with_its_saved_status() =
        runTest(dispatcher) {
            gateway.keys[ProviderView.GEMINI] = "k"
            gateway.statuses[ProviderView.GEMINI] =
                ProviderStatusView(ConnectionStatusView.BAD_KEY, 0u, 42L)
            val vm = ProvidersViewModel(gateway, dispatcher)
            advanceUntilIdle()
            val state = vm.state.value
            assertFalse(state.loading)
            assertFalse(state.failed)
            val gemini = state.items.first { it.provider == ProviderView.GEMINI }
            assertTrue(gemini.hasKey)
            assertEquals(ConnectionStatusView.BAD_KEY, gemini.status)
            assertEquals(42L, gemini.testedAtMs)
            val anthropic = state.items.first { it.provider == ProviderView.ANTHROPIC }
            assertFalse(anthropic.hasKey)
            assertEquals(ConnectionStatusView.UNSET, anthropic.status)
        }

    @Test
    fun a_status_without_a_key_reads_unset() =
        runTest(dispatcher) {
            gateway.statuses[ProviderView.OPEN_AI] =
                ProviderStatusView(ConnectionStatusView.OK, 5u, 1L)
            val vm = ProvidersViewModel(gateway, dispatcher)
            advanceUntilIdle()
            assertEquals(
                ConnectionStatusView.UNSET,
                vm.state.value.items
                    .first { it.provider == ProviderView.OPEN_AI }
                    .status,
            )
        }

    @Test
    fun one_failing_provider_does_not_hide_the_others() =
        runTest(dispatcher) {
            gateway.keys[ProviderView.DEEP_SEEK] = "k"
            gateway.statuses[ProviderView.DEEP_SEEK] =
                ProviderStatusView(ConnectionStatusView.OK, 2u, 3L)
            gateway.failNext = IllegalStateException("boom")
            val vm = ProvidersViewModel(gateway, dispatcher)
            advanceUntilIdle()
            val state = vm.state.value
            assertTrue(state.failed)
            assertEquals(5, state.items.size)
            assertEquals(
                ConnectionStatusView.OK,
                state.items.first { it.provider == ProviderView.DEEP_SEEK }.status,
            )
        }

    @Test
    fun refresh_reads_again() =
        runTest(dispatcher) {
            val vm = ProvidersViewModel(gateway, dispatcher)
            advanceUntilIdle()
            gateway.keys[ProviderView.OPEN_ROUTER] = "k"
            vm.refresh()
            advanceUntilIdle()
            assertTrue(
                vm.state.value.items
                    .first { it.provider == ProviderView.OPEN_ROUTER }
                    .hasKey,
            )
        }
}
