package app.tether.projectarchive

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.IOException

@OptIn(ExperimentalCoroutinesApi::class)
class ExportProjectViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProjectArchiveGateway()
    private val files = FakeArchiveFiles()
    private val destination = ArchiveUri("content://documents/acme-api.zip")

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel() =
        ExportProjectViewModel(
            projectId = "project-7",
            projectName = "acme-api",
            projects = gateway,
            files = files,
            io = dispatcher,
        )

    @Test
    fun a_new_sheet_is_editing_with_ignored_files_left_out() {
        val state = viewModel().state.value
        assertEquals(ExportPhase.Editing, state.phase)
        assertFalse(state.includeIgnored)
        assertEquals("acme-api.zip", state.suggestedName)
    }

    @Test
    fun export_with_the_switch_off_passes_false_and_writes_the_copy_out() {
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()

        val call = gateway.exportCalls.single()
        assertEquals("project-7", call.projectId)
        assertFalse(call.includeIgnored)
        assertEquals(ExportPhase.Done, vm.state.value.phase)
        assertEquals(
            call.destPath,
            files.copiedOut
                .single()
                .first.path,
        )
        assertEquals(destination, files.copiedOut.single().second)
    }

    @Test
    fun export_with_the_switch_on_passes_true() {
        val vm = viewModel()
        vm.onIncludeIgnoredChange(true)
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(gateway.exportCalls.single().includeIgnored)
    }

    @Test
    fun the_switch_cannot_change_while_the_export_runs() {
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        vm.onIncludeIgnoredChange(true)
        dispatcher.scheduler.advanceUntilIdle()
        assertFalse(gateway.exportCalls.single().includeIgnored)
    }

    @Test
    fun the_temp_file_is_deleted_on_success() {
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf(gateway.exportCalls.single().destPath), files.deleted)
    }

    @Test
    fun a_core_failure_shows_its_reason_and_deletes_the_temp_file() {
        gateway.exportResult = ArchiveResult.Failed(ArchiveReason.Unavailable)
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ExportPhase.Failed, vm.state.value.phase)
        assertEquals(ArchiveReason.Unavailable, vm.state.value.reason)
        assertEquals(listOf(gateway.exportCalls.single().destPath), files.deleted)
        assertTrue(files.copiedOut.isEmpty())
    }

    @Test
    fun export_failure_then_try_again_writes_to_the_same_destination() {
        gateway.exportResult = ArchiveResult.Failed(ArchiveReason.Cancelled)
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(ExportPhase.Failed, vm.state.value.phase)

        gateway.exportResult = ArchiveResult.Succeeded(Unit)
        vm.onTryAgain()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ExportPhase.Done, vm.state.value.phase)
        assertEquals(2, gateway.exportCalls.size)
        assertEquals(destination, files.copiedOut.single().second)
        assertEquals(2, files.deleted.size)
    }

    @Test
    fun a_failed_copy_out_shows_the_write_reason_and_deletes_the_temp_file() {
        files.copyOutFailure = IOException("read only")
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ExportPhase.Failed, vm.state.value.phase)
        assertEquals(ArchiveReason.WriteFailed, vm.state.value.reason)
        assertEquals(1, files.deleted.size)
    }

    @Test
    fun a_failed_reserve_shows_the_write_reason_and_calls_no_core() {
        files.reserveFailure = IOException("no cache")
        val vm = viewModel()
        vm.onDestinationChosen(destination)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(ArchiveReason.WriteFailed, vm.state.value.reason)
        assertTrue(gateway.exportCalls.isEmpty())
    }

    @Test
    fun try_again_does_nothing_before_a_failure() {
        val vm = viewModel()
        vm.onTryAgain()
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(gateway.exportCalls.isEmpty())
        assertEquals(ExportPhase.Editing, vm.state.value.phase)
    }
}
