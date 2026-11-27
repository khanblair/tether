package app.tether.workspace

import app.tether.core.generated.ProjectRuntimeException
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
class WorkspaceViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    @Test
    fun it_loads_the_saved_location_and_suggests_the_shared_folder() =
        runTest {
            val gateway = FakeWorkspaceGateway()
            val vm = viewModel(gateway)

            advanceUntilIdle()

            assertEquals(gateway.savedLocation, vm.state.value.location)
            assertEquals("/storage/emulated/0/Tether", vm.state.value.pathInput)
        }

    @Test
    fun the_suggested_path_uses_the_external_storage_root() {
        assertEquals("/storage/emulated/0/Tether", suggestedWorkspaceRoot("/storage/emulated/0"))
    }

    @Test
    fun the_workspace_skip_action_keeps_the_private_default() =
        runTest {
            val gateway =
                FakeWorkspaceGateway(
                    savedLocation = WorkspaceLocation("/private", "/shared", "/shared"),
                )
            val vm = viewModel(gateway)
            advanceUntilIdle()

            vm.usePrivateStorage()
            advanceUntilIdle()

            assertEquals(listOf(null), gateway.savedRoots)
            assertEquals(WorkspaceChoice.Private, vm.state.value.choice)
        }

    @Test
    fun a_custom_folder_failure_stays_visible() =
        runTest {
            val gateway = FakeWorkspaceGateway(nextProblem = WorkspaceProblem.RootNotWritable)
            val vm = viewModel(gateway)
            advanceUntilIdle()
            vm.choose(WorkspaceChoice.Custom)
            vm.onPathChange("/sdcard/Tether")

            vm.save()
            advanceUntilIdle()

            assertEquals(WorkspaceProblem.RootNotWritable, vm.state.value.problem)
            assertFalse(vm.state.value.saving)
        }

    @Test
    fun a_custom_folder_success_updates_the_location() =
        runTest {
            val gateway = FakeWorkspaceGateway()
            var saved = false
            val vm = viewModel(gateway)
            advanceUntilIdle()
            vm.choose(WorkspaceChoice.Custom)
            vm.onPathChange("/sdcard/Tether")

            vm.save { saved = true }
            advanceUntilIdle()

            assertTrue(saved)
            assertEquals(
                WorkspaceLocation(
                    "/data/user/0/app/files/workspaces",
                    "/sdcard/Tether",
                    "/sdcard/Tether",
                ),
                vm.state.value.location,
            )
            assertNull(vm.state.value.problem)
        }

    @Test
    fun a_custom_folder_needs_all_files_access_before_save() =
        runTest {
            val gateway = FakeWorkspaceGateway()
            val vm = viewModel(gateway, access = false)
            advanceUntilIdle()
            vm.choose(WorkspaceChoice.Custom)
            vm.onPathChange("/sdcard/Tether")

            vm.save()

            assertTrue(vm.state.value.permissionRequired)
            assertTrue(gateway.savedRoots.isEmpty())
        }

    @Test
    fun cheap_path_checks_happen_as_the_user_types() {
        assertEquals(WorkspaceProblem.EmptyPath, validateWorkspacePath(""))
        assertEquals(WorkspaceProblem.RootNotAbsolute, validateWorkspacePath("sdcard/Tether"))
        assertNull(validateWorkspacePath("/sdcard/Tether"))
    }

    private fun viewModel(
        gateway: FakeWorkspaceGateway,
        access: Boolean = true,
    ) = WorkspaceViewModel(
        gateway = gateway,
        sharedFolderAllowed = true,
        initialAllFilesAccessGranted = access,
        suggestedPath = "/storage/emulated/0/Tether",
        io = dispatcher,
    )
}

class WorkspaceProblemMappingTest {
    @Test
    fun every_core_root_error_maps_to_its_workspace_problem() {
        val cases =
            listOf(
                ProjectRuntimeException.RootNotAbsolute() to WorkspaceProblem.RootNotAbsolute,
                ProjectRuntimeException.RootCannotBeCreated() to
                    WorkspaceProblem.RootCannotBeCreated,
                ProjectRuntimeException.RootNotDirectory() to WorkspaceProblem.RootNotDirectory,
                ProjectRuntimeException.RootIsSymlink() to WorkspaceProblem.RootIsSymlink,
                ProjectRuntimeException.RootNotWritable() to WorkspaceProblem.RootNotWritable,
                ProjectRuntimeException.RootInsidePrivateRoot() to
                    WorkspaceProblem.RootInsidePrivateRoot,
                ProjectRuntimeException.RootIsFilesystemRoot() to
                    WorkspaceProblem.RootIsFilesystemRoot,
                ProjectRuntimeException.RootInsideProject() to WorkspaceProblem.RootInsideProject,
                ProjectRuntimeException.RootUnavailable() to WorkspaceProblem.RootUnavailable,
            )

        cases.forEach { (error, expected) -> assertEquals(expected, error.toWorkspaceProblem()) }
    }
}
