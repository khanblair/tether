package app.tether.session.approval

import app.tether.session.agent.AgentLoopException
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.ApprovalUi
import app.tether.session.agent.FakeAgentLoopGateway
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
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
class ApprovalViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeAgentLoopGateway()
    private val waiting =
        MutableStateFlow(
            listOf(
                ApprovalUi("t1", "run", "ls -la"),
                ApprovalUi("t2", "edit_file", "src/app.rs"),
                ApprovalUi("t3", "git_push", "push feature/work to origin"),
            ),
        )
    private val resolved = mutableListOf<String>()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): ApprovalViewModel {
        val vm =
            ApprovalViewModel(
                sessionId = "s1",
                approvals = waiting,
                gateway = gateway,
                onResolved = { resolved += it },
                io = dispatcher,
            )
        advanceUntilIdle()
        return vm
    }

    @Test
    fun the_state_lists_the_waiting_approvals() =
        runTest(dispatcher) {
            val vm = model()
            assertEquals(waiting.value, vm.state.value.approvals)
            assertNull(vm.state.value.expandedId)
            assertNull(vm.state.value.sheet)
            assertFalse(vm.state.value.busy)
        }

    @Test
    fun allow_calls_approve_without_always_and_reports_the_id_once() =
        runTest(dispatcher) {
            val vm = model()
            vm.allow("t1")
            advanceUntilIdle()
            assertEquals(listOf("approve:s1,t1,false"), gateway.log)
            assertEquals(listOf("t1"), resolved)
            assertFalse(vm.state.value.busy)
        }

    @Test
    fun a_second_tap_while_a_call_runs_does_nothing() =
        runTest(dispatcher) {
            val vm = model()
            vm.allow("t1")
            vm.allow("t1")
            advanceUntilIdle()
            assertEquals(listOf("approve:s1,t1,false"), gateway.log)
            assertEquals(listOf("t1"), resolved)
        }

    @Test
    fun always_calls_approve_with_always_when_the_call_has_a_rule() =
        runTest(dispatcher) {
            val vm = model()
            vm.openAlwaysAllow("t1")
            advanceUntilIdle()
            assertEquals(ApprovalSheet.AlwaysAllow("t1"), vm.state.value.sheet)
            vm.confirmAlwaysAllow("t1")
            advanceUntilIdle()
            assertEquals(listOf("approve:s1,t1,true"), gateway.log)
            assertEquals(listOf("t1"), resolved)
            assertNull(vm.state.value.sheet)
        }

    @Test
    fun always_does_nothing_when_the_call_has_no_rule() =
        runTest(dispatcher) {
            val vm = model()
            vm.openAlwaysAllow("t3")
            assertNull(vm.state.value.sheet)
            vm.confirmAlwaysAllow("t3")
            advanceUntilIdle()
            assertTrue(gateway.log.isEmpty())
            assertTrue(resolved.isEmpty())
        }

    @Test
    fun deny_passes_the_trimmed_reason() =
        runTest(dispatcher) {
            val vm = model()
            vm.openDeny("t1")
            vm.editReason("  use the sandbox  ")
            vm.confirmDeny("t1")
            advanceUntilIdle()
            assertEquals(listOf("deny:s1,t1,use the sandbox"), gateway.log)
            assertEquals(listOf("t1"), resolved)
            assertNull(vm.state.value.sheet)
            assertEquals("", vm.state.value.reason)
        }

    @Test
    fun deny_with_a_blank_reason_passes_null() =
        runTest(dispatcher) {
            val vm = model()
            vm.openDeny("t1")
            vm.editReason("   ")
            vm.confirmDeny("t1")
            advanceUntilIdle()
            assertEquals(listOf("deny:s1,t1,null"), gateway.log)
        }

    @Test
    fun the_reason_is_cut_to_the_maximum_length() =
        runTest(dispatcher) {
            val vm = model()
            vm.openDeny("t1")
            vm.editReason("a".repeat(MAX_DENY_REASON_CHARS + 50))
            advanceUntilIdle()
            assertEquals(MAX_DENY_REASON_CHARS, vm.state.value.reason.length)
        }

    @Test
    fun a_failed_call_sets_the_failure_clears_busy_and_reports_nothing() =
        runTest(dispatcher) {
            val vm = model()
            gateway.failNext = AgentLoopException(AgentLoopFailure.Busy)
            vm.allow("t1")
            advanceUntilIdle()
            assertEquals(AgentLoopFailure.Busy, vm.state.value.failure)
            assertFalse(vm.state.value.busy)
            assertTrue(resolved.isEmpty())
        }

    @Test
    fun a_new_call_clears_the_old_failure() =
        runTest(dispatcher) {
            val vm = model()
            gateway.failNext = AgentLoopException(AgentLoopFailure.Unavailable)
            vm.allow("t1")
            advanceUntilIdle()
            vm.allow("t1")
            advanceUntilIdle()
            assertNull(vm.state.value.failure)
            assertEquals(listOf("t1"), resolved)
        }

    @Test
    fun toggling_expanded_shows_and_hides_the_same_approval() =
        runTest(dispatcher) {
            val vm = model()
            vm.toggleExpanded("t1")
            advanceUntilIdle()
            assertEquals("t1", vm.state.value.expandedId)
            vm.toggleExpanded("t1")
            advanceUntilIdle()
            assertNull(vm.state.value.expandedId)
        }

    @Test
    fun the_sheet_closes_when_its_approval_leaves_the_list() =
        runTest(dispatcher) {
            val vm = model()
            vm.toggleExpanded("t1")
            vm.openDeny("t1")
            waiting.value = waiting.value.drop(1)
            advanceUntilIdle()
            assertNull(vm.state.value.sheet)
            assertNull(vm.state.value.expandedId)
        }

    @Test
    fun closing_the_sheet_drops_the_reason() =
        runTest(dispatcher) {
            val vm = model()
            vm.openDeny("t2")
            vm.editReason("too risky")
            vm.closeSheet()
            assertNull(vm.state.value.sheet)
            assertEquals("", vm.state.value.reason)
        }
}
