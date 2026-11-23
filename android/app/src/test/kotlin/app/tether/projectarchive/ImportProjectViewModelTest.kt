package app.tether.projectarchive

import app.tether.home.ProjectNameProblem
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
class ImportProjectViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProjectArchiveGateway()
    private val files = FakeArchiveFiles()
    private val zip = ArchiveUri("content://downloads/api-server.zip")

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel() = ImportProjectViewModel(gateway, files, io = dispatcher)

    /** A view model with [zip] picked and the picked state settled. */
    private fun pickedViewModel(): ImportProjectViewModel {
        val vm = viewModel()
        vm.onPicked(zip)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    @Test
    fun a_new_screen_is_idle_with_import_off() {
        val state = viewModel().state.value
        assertEquals(ImportPhase.Idle, state.phase)
        assertFalse(state.canImport)
        assertNull(state.fileName)
    }

    @Test
    fun picking_a_file_shows_its_name_and_size_and_prefills_the_project_name() {
        val state = pickedViewModel().state.value
        assertEquals(ImportPhase.Picked, state.phase)
        assertEquals("api-server.zip", state.fileName)
        assertEquals(2_048L, state.sizeBytes)
        assertEquals("api-server", state.name)
        assertTrue(state.nameUsable)
        assertTrue(state.canImport)
        assertFalse(state.tooLarge)
    }

    @Test
    fun pick_then_import_success_opens_the_new_project_and_deletes_the_copy() {
        val vm = pickedViewModel()
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()

        val state = vm.state.value
        assertEquals(ImportPhase.Done, state.phase)
        assertEquals("project-new", state.projectId)
        assertEquals(
            listOf(FakeProjectArchiveGateway.ImportCall(files.deleted.single(), "api-server")),
            gateway.importCalls,
        )
        assertEquals(listOf(zip), files.copiedIn)
    }

    @Test
    fun the_copy_is_deleted_when_the_core_fails() {
        gateway.importResult = ArchiveResult.Failed(ArchiveReason.TooLarge)
        val vm = pickedViewModel()
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ImportPhase.Failed, vm.state.value.phase)
        assertEquals(ArchiveReason.TooLarge, vm.state.value.reason)
        assertEquals(gateway.importCalls.single().zipPath, files.deleted.single())
    }

    @Test
    fun a_core_failure_keeps_the_name_so_try_again_can_run() {
        gateway.importResult = ArchiveResult.Failed(ArchiveReason.Unexpected)
        val vm = pickedViewModel()
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals("api-server", vm.state.value.name)
        assertTrue(vm.state.value.canImport)

        gateway.importResult = ArchiveResult.Succeeded("project-retry")
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ImportPhase.Done, vm.state.value.phase)
        assertEquals("project-retry", vm.state.value.projectId)
        assertEquals(2, gateway.importCalls.size)
        assertEquals(2, files.deleted.size)
    }

    @Test
    fun an_empty_name_keeps_import_off_without_a_message() {
        val vm = pickedViewModel()
        vm.onNameChange("   ")
        assertFalse(vm.state.value.canImport)
        assertNull(vm.state.value.nameProblem)
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(gateway.importCalls.isEmpty())
    }

    @Test
    fun a_name_over_80_characters_shows_the_length_problem() {
        val vm = pickedViewModel()
        vm.onNameChange("a".repeat(81))
        assertEquals(ProjectNameProblem.TooLong, vm.state.value.nameProblem)
        assertFalse(vm.state.value.canImport)
    }

    @Test
    fun a_name_with_a_newline_shows_the_control_problem() {
        val vm = pickedViewModel()
        vm.onNameChange("api\nserver")
        assertEquals(ProjectNameProblem.ControlCharacter, vm.state.value.nameProblem)
        assertFalse(vm.state.value.canImport)
    }

    @Test
    fun a_file_over_200_mib_is_refused_before_any_copy() {
        files.sizeValue = ARCHIVE_IMPORT_LIMIT_BYTES + 1
        val vm = pickedViewModel()
        assertTrue(vm.state.value.tooLarge)
        assertFalse(vm.state.value.canImport)

        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(files.copiedIn.isEmpty())
        assertTrue(gateway.importCalls.isEmpty())
        assertEquals(ImportPhase.Picked, vm.state.value.phase)
    }

    @Test
    fun a_file_of_exactly_200_mib_is_allowed() {
        files.sizeValue = ARCHIVE_IMPORT_LIMIT_BYTES
        val state = pickedViewModel().state.value
        assertFalse(state.tooLarge)
        assertTrue(state.canImport)
    }

    @Test
    fun an_unknown_size_is_not_refused() {
        files.sizeValue = null
        val state = pickedViewModel().state.value
        assertFalse(state.tooLarge)
        assertNull(state.sizeBytes)
        assertTrue(state.canImport)
    }

    @Test
    fun a_failed_read_of_the_file_shows_the_read_reason_and_calls_no_core() {
        files.copyInFailure = IOException("gone")
        val vm = pickedViewModel()
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ImportPhase.Failed, vm.state.value.phase)
        assertEquals(ArchiveReason.ReadFailed, vm.state.value.reason)
        assertTrue(gateway.importCalls.isEmpty())
        assertTrue(files.deleted.isEmpty())
    }

    @Test
    fun a_core_call_that_throws_becomes_the_unavailable_reason() {
        gateway.importResult = ArchiveResult.Failed(ArchiveReason.Unavailable)
        val vm = pickedViewModel()
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(ArchiveReason.Unavailable, vm.state.value.reason)
    }

    @Test
    fun cancel_while_importing_calls_cancel_shows_no_error_and_returns_to_picked() {
        val vm = pickedViewModel()
        gateway.beforeImport = {
            vm.onCancel()
            assertEquals(ImportPhase.Picked, vm.state.value.phase)
        }
        gateway.importResult = ArchiveResult.Failed(ArchiveReason.Cancelled)
        vm.onImport()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(1, gateway.cancelCount)
        assertEquals(ImportPhase.Picked, vm.state.value.phase)
        assertNull(vm.state.value.reason)
        assertEquals(1, files.deleted.size)
    }

    @Test
    fun cancel_before_the_core_call_skips_the_core() {
        val vm = pickedViewModel()
        vm.onImport()
        vm.onCancel()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ImportPhase.Picked, vm.state.value.phase)
        assertNull(vm.state.value.reason)
        assertEquals(1, gateway.cancelCount)
        assertTrue(gateway.importCalls.isEmpty())
    }

    @Test
    fun cancel_does_nothing_when_no_import_runs() {
        val vm = pickedViewModel()
        vm.onCancel()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(0, gateway.cancelCount)
        assertEquals(ImportPhase.Picked, vm.state.value.phase)
    }

    @Test
    fun a_cancelled_picker_leaves_the_screen_as_it_was() {
        val vm = pickedViewModel()
        val before = vm.state.value
        vm.onPickCancelled()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(before, vm.state.value)
    }

    @Test
    fun a_new_pick_replaces_the_file_and_its_name() {
        files.nameValue = "second-app.zip"
        val vm = pickedViewModel()
        vm.onPicked(ArchiveUri("content://downloads/second-app.zip"))
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals("second-app", vm.state.value.name)
        assertEquals("second-app.zip", vm.state.value.fileName)
    }
}
