package app.tether.home

import app.tether.session.FakeSessionGateway
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
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SessionRenameViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeSessionGateway()
    private var renamedCalls = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun vm() = SessionRenameViewModel(gateway, io = dispatcher)

    @Test
    fun start_fills_the_field_with_the_current_title() {
        val vm = vm()
        vm.start("s1", "Fix login")
        val state = vm.state.value
        assertEquals("s1", state.sessionId)
        assertEquals("Fix login", state.currentTitle)
        assertEquals("Fix login", state.title)
        assertTrue(state.canSave)
    }

    @Test
    fun save_sends_the_trimmed_title_and_closes_on_success() =
        runTest(dispatcher) {
            val vm = vm()
            vm.start("s1", "Old")
            vm.onTitleChange("  New name  ")
            vm.save { renamedCalls++ }
            advanceUntilIdle()
            assertEquals(listOf("s1" to "New name"), gateway.renamed)
            assertEquals(1, renamedCalls)
            assertNull(vm.state.value.sessionId)
        }

    @Test
    fun a_blank_title_is_never_sent_and_cannot_be_saved() =
        runTest(dispatcher) {
            val vm = vm()
            vm.start("s1", "Old")
            vm.onTitleChange("   ")
            assertFalse(vm.state.value.canSave)
            vm.save { renamedCalls++ }
            advanceUntilIdle()
            assertTrue(gateway.renamed.isEmpty())
            assertEquals(0, renamedCalls)
            assertEquals("s1", vm.state.value.sessionId)
        }

    @Test
    fun a_failed_save_keeps_the_sheet_open_with_the_failure_flag() =
        runTest(dispatcher) {
            gateway.renameError = IllegalStateException("core down")
            val vm = vm()
            vm.start("s1", "Old")
            vm.onTitleChange("New")
            vm.save { renamedCalls++ }
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals("s1", state.sessionId)
            assertTrue(state.failed)
            assertFalse(state.saving)
            assertEquals(0, renamedCalls)
        }

    @Test
    fun editing_the_title_clears_the_failure_flag() =
        runTest(dispatcher) {
            gateway.renameError = IllegalStateException("core down")
            val vm = vm()
            vm.start("s1", "Old")
            vm.save { renamedCalls++ }
            advanceUntilIdle()
            assertTrue(vm.state.value.failed)
            vm.onTitleChange("Newer")
            assertFalse(vm.state.value.failed)
        }

    @Test
    fun dismiss_closes_the_rename_without_saving() =
        runTest(dispatcher) {
            val vm = vm()
            vm.start("s1", "Old")
            vm.onTitleChange("New")
            vm.dismiss()
            vm.save { renamedCalls++ }
            advanceUntilIdle()
            assertNull(vm.state.value.sessionId)
            assertTrue(gateway.renamed.isEmpty())
        }

    @Test
    fun save_with_no_rename_open_does_nothing() =
        runTest(dispatcher) {
            vm().save { renamedCalls++ }
            advanceUntilIdle()
            assertTrue(gateway.renamed.isEmpty())
            assertEquals(0, renamedCalls)
        }
}
