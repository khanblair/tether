package app.tether.settings.mcp

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
class McpServersViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeMcpGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): McpServersViewModel {
        val vm = McpServersViewModel(gateway, dispatcher)
        advanceUntilIdle()
        return vm
    }

    @Test
    fun the_list_shows_each_saved_server() =
        runTest(dispatcher) {
            gateway.serverList += sampleServer("docs-search")
            gateway.serverList += sampleServer("wiki", enabled = false)
            val state = model().state.value
            assertFalse(state.loading)
            assertFalse(state.loadFailed)
            assertEquals(listOf("docs-search", "wiki"), state.servers.map { it.id })
        }

    @Test
    fun an_empty_list_is_empty_and_not_failed() =
        runTest(dispatcher) {
            val state = model().state.value
            assertFalse(state.loading)
            assertFalse(state.loadFailed)
            assertTrue(state.servers.isEmpty())
        }

    @Test
    fun a_failed_load_says_so_and_stops_loading() =
        runTest(dispatcher) {
            gateway.failNext = McpFailure(McpProblem.Unavailable)
            val state = model().state.value
            assertFalse(state.loading)
            assertTrue(state.loadFailed)
            assertTrue(state.servers.isEmpty())
        }

    @Test
    fun turning_a_server_off_calls_the_gateway_and_reads_the_list_again() =
        runTest(dispatcher) {
            gateway.serverList += sampleServer("docs-search")
            val vm = model()
            vm.setEnabled("docs-search", false)
            advanceUntilIdle()
            assertTrue("setEnabled:docs-search:false" in gateway.calls)
            assertFalse(
                vm.state.value.servers
                    .single()
                    .enabled,
            )
            assertNull(vm.state.value.actionProblem)
        }

    @Test
    fun a_failed_toggle_shows_its_problem_and_the_banner_can_be_dismissed() =
        runTest(dispatcher) {
            gateway.serverList += sampleServer("docs-search")
            val vm = model()
            gateway.failNext = McpFailure(McpProblem.NotFound)
            vm.setEnabled("docs-search", false)
            advanceUntilIdle()
            assertEquals(McpProblem.NotFound, vm.state.value.actionProblem)
            vm.dismissProblem()
            assertNull(vm.state.value.actionProblem)
        }

    @Test
    fun a_test_that_passes_keeps_the_tool_count_under_the_row() =
        runTest(dispatcher) {
            gateway.serverList += sampleServer("docs-search")
            gateway.testToolCount = 5
            val vm = model()
            vm.test("docs-search")
            advanceUntilIdle()
            assertEquals(McpTestResult.Connected(5), vm.state.value.testResults["docs-search"])
            assertTrue(
                vm.state.value.testing
                    .isEmpty(),
            )
        }

    @Test
    fun a_test_that_fails_keeps_a_failed_result_and_stops_testing() =
        runTest(dispatcher) {
            gateway.serverList += sampleServer("docs-search")
            val vm = model()
            gateway.failNext = McpFailure(McpProblem.TestFailed)
            vm.test("docs-search")
            advanceUntilIdle()
            assertEquals(McpTestResult.Failed, vm.state.value.testResults["docs-search"])
            assertTrue(
                vm.state.value.testing
                    .isEmpty(),
            )
        }

    @Test
    fun reload_reads_the_list_again() =
        runTest(dispatcher) {
            val vm = model()
            gateway.serverList += sampleServer("wiki")
            vm.reload()
            advanceUntilIdle()
            assertEquals(
                listOf("wiki"),
                vm.state.value.servers
                    .map { it.id },
            )
        }
}
