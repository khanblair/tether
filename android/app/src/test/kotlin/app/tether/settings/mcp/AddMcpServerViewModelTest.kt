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
class AddMcpServerViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeMcpGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): AddMcpServerViewModel {
        val vm = AddMcpServerViewModel(gateway, dispatcher)
        advanceUntilIdle()
        return vm
    }

    private fun AddMcpServerViewModel.fillValidForm() {
        onNameChange("docs-search")
        onUrlChange("https://mcp.example.com/docs")
    }

    @Test
    fun add_stays_disabled_until_name_and_address_are_valid() =
        runTest(dispatcher) {
            val vm = model()
            assertFalse(vm.state.value.canAdd)
            vm.onNameChange("Docs Search")
            vm.onUrlChange("https://mcp.example.com")
            assertFalse(vm.state.value.canAdd)
            vm.onNameChange("docs-search")
            assertTrue(vm.state.value.canAdd)
        }

    @Test
    fun a_header_row_with_a_bad_name_or_value_blocks_add() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            vm.addHeader()
            vm.onHeaderNameChange(0, "X_Team")
            vm.onHeaderValueChange(0, "value")
            assertFalse(vm.state.value.canAdd)
            vm.onHeaderNameChange(0, "X-Team")
            assertTrue(vm.state.value.canAdd)
            vm.onHeaderValueChange(0, "line\nbreak")
            assertFalse(vm.state.value.canAdd)
        }

    @Test
    fun the_form_takes_at_most_8_headers() =
        runTest(dispatcher) {
            val vm = model()
            repeat(MCP_MAX_HEADERS + 2) { vm.addHeader() }
            assertEquals(MCP_MAX_HEADERS, vm.state.value.headers.size)
            assertFalse(vm.state.value.canAddHeader)
        }

    @Test
    fun removing_a_header_row_removes_only_that_row() =
        runTest(dispatcher) {
            val vm = model()
            vm.addHeader()
            vm.addHeader()
            vm.onHeaderNameChange(0, "First")
            vm.onHeaderNameChange(1, "Second")
            vm.removeHeader(0)
            assertEquals(
                listOf("Second"),
                vm.state.value.headers
                    .map { it.name },
            )
        }

    @Test
    fun a_successful_add_sends_the_values_once_then_clears_them_and_runs_one_test() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            vm.addHeader()
            vm.onHeaderNameChange(0, "Authorization")
            vm.onHeaderValueChange(0, "secret-token-123")
            vm.add()
            advanceUntilIdle()

            val request = gateway.addRequests.single()
            assertEquals("docs-search", request.name)
            assertEquals(listOf("Authorization" to "secret-token-123"), request.headers)
            assertEquals(1, gateway.calls.count { it == "test:docs-search" })

            val state = vm.state.value
            assertEquals("docs-search", state.addedId)
            assertTrue(state.headers.isEmpty())
            assertFalse(state.testing)
            assertEquals(
                McpTestResult.Connected(FakeMcpGateway.DEFAULT_TOOL_COUNT),
                state.testResult,
            )
            assertFalse(state.toString().contains("secret-token-123"))
        }

    @Test
    fun a_test_that_fails_after_the_save_keeps_the_server_saved_and_shows_failed() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            gateway.failTest = McpFailure(McpProblem.TestFailed)
            vm.add()
            advanceUntilIdle()

            val state = vm.state.value
            assertEquals("docs-search", state.addedId)
            assertEquals(McpTestResult.Failed, state.testResult)
            assertNull(state.problem)
            assertFalse(state.testing)
            assertEquals(1, gateway.addRequests.size)
        }

    @Test
    fun the_test_runs_once_and_only_after_the_save() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            gateway.nextId = "wiki"
            vm.add()
            advanceUntilIdle()
            assertEquals(
                listOf("add", "test:wiki"),
                gateway.calls.filter {
                    it == "add" ||
                        it.startsWith("test")
                },
            )
            assertEquals("wiki", vm.state.value.addedId)
        }

    @Test
    fun a_failed_save_keeps_the_form_and_its_values_but_hides_them_in_state_text() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            vm.addHeader()
            vm.onHeaderNameChange(0, "Authorization")
            vm.onHeaderValueChange(0, "secret-token-456")
            gateway.failNext = McpFailure(McpProblem.InvalidUrl)
            vm.add()
            advanceUntilIdle()

            val state = vm.state.value
            assertEquals(McpProblem.InvalidUrl, state.problem)
            assertNull(state.addedId)
            assertFalse(state.adding)
            assertEquals("https://mcp.example.com/docs", state.url)
            assertEquals("secret-token-456", state.headers.single().value)
            assertFalse(state.toString().contains("secret-token-456"))
            assertFalse(
                state.headers
                    .single()
                    .toString()
                    .contains("secret-token-456"),
            )
            assertFalse("test" in gateway.calls.map { it.substringBefore(':') })
        }

    @Test
    fun the_problem_banner_clears_when_the_person_edits_the_form() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            gateway.failNext = McpFailure(McpProblem.Unavailable)
            vm.add()
            advanceUntilIdle()
            assertEquals(McpProblem.Unavailable, vm.state.value.problem)
            vm.onNameChange("docs-search-2")
            assertNull(vm.state.value.problem)
        }

    @Test
    fun the_form_is_read_only_once_the_server_is_saved() =
        runTest(dispatcher) {
            val vm = model()
            vm.fillValidForm()
            vm.add()
            advanceUntilIdle()
            vm.onNameChange("changed")
            vm.addHeader()
            assertEquals("docs-search", vm.state.value.name)
            assertTrue(
                vm.state.value.headers
                    .isEmpty(),
            )
        }

    @Test
    fun a_header_draft_hides_its_value_in_text() {
        val draft = HeaderDraft(name = "Authorization", value = "secret-token-789")
        assertFalse(draft.toString().contains("secret-token-789"))
        assertTrue(draft.toString().contains("Authorization"))
    }

    @Test
    fun a_failure_without_a_cause_text_maps_to_its_problem_only() {
        val failure = McpFailure(McpProblem.TestFailed)
        assertNull(failure.cause)
        assertEquals("", failure.message ?: "")
        assertEquals(McpProblem.Unavailable, IllegalStateException("reason=token").mcpProblem())
        assertEquals(McpProblem.TestFailed, failure.mcpProblem())
    }
}
