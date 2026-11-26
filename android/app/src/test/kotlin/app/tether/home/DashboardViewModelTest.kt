package app.tether.home

import app.tether.core.generated.ProviderView
import app.tether.design.components.SessionState
import app.tether.providers.FakeProviderGateway
import app.tether.providers.ProviderGateway
import app.tether.updates.UpdatesUiState
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.time.Instant
import java.time.ZoneId
import java.util.Locale

@OptIn(ExperimentalCoroutinesApi::class)
class DashboardViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProviderGateway()
    private val now = Instant.parse("2026-11-19T12:00:00Z").toEpochMilli()
    private val updates =
        MutableStateFlow(UpdatesUiState(currentVersion = "0.0.3", installsInApp = true))

    private val sources =
        HomeSources(
            projects =
                object : ProjectsSource {
                    override fun projects() =
                        listOf(ProjectSummary("p", "p", "main", 0, now, false))
                },
            sessions =
                object : SessionsSource {
                    override fun sessions() =
                        listOf(SessionSummary("s", "t", "p", SessionState.Waiting, now))

                    override fun archivedSessions() = emptyList<SessionSummary>()
                },
            usage =
                object : UsageSource {
                    override fun usage() = UsageSnapshot.Empty
                },
        )

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel() =
        DashboardViewModel(
            sources = sources,
            providers = gateway,
            updates = updates,
            clock = { now },
            zone = ZoneId.of("UTC"),
            locale = Locale.US,
            io = dispatcher,
        )

    @Test
    fun the_providers_are_null_until_the_dispatcher_runs_and_then_filled() =
        runTest(dispatcher) {
            gateway.keys[ProviderView.ANTHROPIC] = "key"
            val vm = viewModel()
            assertNull(vm.state.value.providers)
            advanceUntilIdle()
            val providers = vm.state.value.providers
            assertEquals(ProviderView.entries, providers?.map { it.provider })
            assertTrue(providers?.first { it.provider == ProviderView.ANTHROPIC }?.hasKey == true)
        }

    @Test
    fun a_core_failure_keeps_the_providers_hidden_and_the_rest_loaded() =
        runTest(dispatcher) {
            val failing =
                object : ProviderGateway by gateway {
                    override fun hasKey(provider: ProviderView): Boolean = error("core down")
                }
            val vm =
                DashboardViewModel(
                    sources = sources,
                    providers = failing,
                    updates = updates,
                    clock = { now },
                    zone = ZoneId.of("UTC"),
                    locale = Locale.US,
                    io = dispatcher,
                )
            advanceUntilIdle()
            assertNull(vm.state.value.providers)
            assertEquals(1, vm.state.value.projectCount)
        }

    @Test
    fun refresh_picks_up_a_changed_key_state() =
        runTest(dispatcher) {
            val vm = viewModel()
            advanceUntilIdle()
            gateway.keys[ProviderView.OPEN_AI] = "key"
            vm.refresh()
            advanceUntilIdle()
            val openAi =
                vm.state.value.providers
                    ?.first { it.provider == ProviderView.OPEN_AI }
            assertTrue(openAi?.hasKey == true)
        }

    @Test
    fun the_update_flow_changes_the_attention_list() =
        runTest(dispatcher) {
            val vm = viewModel()
            advanceUntilIdle()
            assertEquals(1, vm.state.value.attention.size)
            updates.value =
                UpdatesUiState(
                    currentVersion = "0.0.3",
                    installsInApp = true,
                    knownNewerVersion = "0.0.4",
                )
            advanceUntilIdle()
            assertEquals(2, vm.state.value.attention.size)
        }
}
