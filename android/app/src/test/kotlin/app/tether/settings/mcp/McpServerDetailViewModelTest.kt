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
class McpServerDetailViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeMcpGateway()
    private val serverId = "docs-search"

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): McpServerDetailViewModel {
        val vm = McpServerDetailViewModel(serverId, gateway, dispatcher)
        advanceUntilIdle()
        return vm
    }

    private fun withTools(vararg tools: McpTool) {
        gateway.serverList += sampleServer(serverId, tools = tools.toList())
    }

    @Test
    fun the_screen_loads_the_server_and_its_tools() =
        runTest(dispatcher) {
            withTools(McpTool("search_docs", "Search.", true, McpApproval.Ask))
            val state = model().state.value
            assertFalse(state.loading)
            assertNull(state.loadProblem)
            assertEquals(serverId, state.server?.id)
            assertEquals(
                "search_docs",
                state.server
                    ?.tools
                    ?.single()
                    ?.name,
            )
        }

    @Test
    fun a_server_that_is_gone_is_not_found() =
        runTest(dispatcher) {
            val state = model().state.value
            assertFalse(state.loading)
            assertNull(state.server)
            assertEquals(McpProblem.NotFound, state.loadProblem)
        }

    @Test
    fun a_failed_read_is_a_load_problem() =
        runTest(dispatcher) {
            gateway.failNext = McpFailure(McpProblem.Unavailable)
            val state = model().state.value
            assertNull(state.server)
            assertEquals(McpProblem.Unavailable, state.loadProblem)
        }

    @Test
    fun turning_a_tool_on_or_off_keeps_its_approval() =
        runTest(dispatcher) {
            withTools(McpTool("search_docs", "Search.", true, McpApproval.Auto))
            val vm = model()
            vm.setToolEnabled("search_docs", false)
            advanceUntilIdle()
            assertEquals(
                ToolCall(serverId, "search_docs", false, "auto"),
                gateway.toolCalls.single(),
            )
            assertEquals(
                false,
                vm.state.value.server
                    ?.tools
                    ?.single()
                    ?.enabled,
            )
            assertFalse(vm.state.value.busy)
        }

    @Test
    fun changing_the_approval_keeps_the_enabled_flag_and_sends_lowercase_words() =
        runTest(dispatcher) {
            withTools(McpTool("search_docs", "Search.", false, McpApproval.Ask))
            val vm = model()
            vm.setToolApproval("search_docs", McpApproval.Auto)
            advanceUntilIdle()
            assertEquals(
                ToolCall(serverId, "search_docs", false, "auto"),
                gateway.toolCalls.single(),
            )
            assertEquals(
                McpApproval.Auto,
                vm.state.value.server
                    ?.tools
                    ?.single()
                    ?.approval,
            )

            vm.setToolApproval("search_docs", McpApproval.Ask)
            advanceUntilIdle()
            assertEquals("ask", gateway.toolCalls.last().approval)
        }

    @Test
    fun a_change_to_a_tool_that_is_not_listed_sends_nothing() =
        runTest(dispatcher) {
            withTools(McpTool("search_docs", "Search.", true, McpApproval.Ask))
            val vm = model()
            vm.setToolEnabled("missing_tool", false)
            advanceUntilIdle()
            assertTrue(gateway.toolCalls.isEmpty())
        }

    @Test
    fun a_failed_tool_change_keeps_the_screen_and_sets_the_problem() =
        runTest(dispatcher) {
            withTools(McpTool("search_docs", "Search.", true, McpApproval.Ask))
            val vm = model()
            gateway.failNext = McpFailure(McpProblem.InvalidApproval)
            vm.setToolApproval("search_docs", McpApproval.Auto)
            advanceUntilIdle()
            assertEquals(McpProblem.InvalidApproval, vm.state.value.problem)
            assertEquals(
                serverId,
                vm.state.value.server
                    ?.id,
            )
            assertFalse(vm.state.value.busy)
        }

    @Test
    fun refresh_calls_the_gateway_and_reads_the_server_again() =
        runTest(dispatcher) {
            withTools()
            val vm = model()
            vm.refreshTools()
            advanceUntilIdle()
            assertTrue("refreshTools:$serverId" in gateway.calls)
            assertFalse(vm.state.value.busy)
        }

    @Test
    fun remove_asks_first_and_cancel_removes_nothing() =
        runTest(dispatcher) {
            withTools()
            val vm = model()
            vm.askRemove()
            assertTrue(vm.state.value.confirmingRemove)
            vm.cancelRemove()
            assertFalse(vm.state.value.confirmingRemove)
            assertFalse(gateway.calls.any { it.startsWith("remove") })
            assertFalse(vm.state.value.removed)
        }

    @Test
    fun confirming_remove_deletes_the_server_and_marks_the_screen_done() =
        runTest(dispatcher) {
            withTools()
            val vm = model()
            vm.askRemove()
            vm.confirmRemove()
            advanceUntilIdle()
            assertTrue("remove:$serverId" in gateway.calls)
            assertTrue(vm.state.value.removed)
            assertFalse(vm.state.value.confirmingRemove)
        }

    @Test
    fun a_failed_removal_stays_on_the_screen_with_a_problem() =
        runTest(dispatcher) {
            withTools()
            val vm = model()
            vm.askRemove()
            gateway.failNext = McpFailure(McpProblem.Unavailable)
            vm.confirmRemove()
            advanceUntilIdle()
            assertFalse(vm.state.value.removed)
            assertEquals(McpProblem.Unavailable, vm.state.value.problem)
            vm.dismissProblem()
            assertNull(vm.state.value.problem)
        }
}
