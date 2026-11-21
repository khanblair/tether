package app.tether.home

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class NewProjectViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeProjectGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel() = NewProjectViewModel(gateway, io = dispatcher)

    @Test
    fun a_new_screen_is_editing_with_create_off() {
        val state = viewModel().state.value
        assertEquals(NewProjectPhase.Editing, state.phase)
        assertFalse(state.canCreate)
        assertNull(state.problem)
    }

    @Test
    fun a_blank_name_keeps_create_off_without_a_message() {
        val vm = viewModel()
        vm.onNameChange("   ")
        assertFalse(vm.state.value.canCreate)
        assertNull(vm.state.value.problem)
    }

    @Test
    fun a_name_over_the_limit_shows_the_length_problem() {
        val vm = viewModel()
        vm.onNameChange("a".repeat(81))
        assertEquals(ProjectNameProblem.TooLong, vm.state.value.problem)
        assertFalse(vm.state.value.canCreate)
    }

    @Test
    fun a_name_with_a_newline_shows_the_control_problem() {
        val vm = viewModel()
        vm.onNameChange("api\nserver")
        assertEquals(ProjectNameProblem.ControlCharacter, vm.state.value.problem)
    }

    @Test
    fun a_usable_name_turns_create_on() {
        val vm = viewModel()
        vm.onNameChange("  Blog  ")
        assertTrue(vm.state.value.canCreate)
        assertNull(vm.state.value.problem)
    }

    @Test
    fun create_makes_the_project_and_reports_its_id() {
        val vm = viewModel()
        vm.onNameChange("Blog")
        vm.create()
        assertEquals(NewProjectPhase.Creating, vm.state.value.phase)
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(NewProjectPhase.Created, vm.state.value.phase)
        assertEquals("project-1", vm.state.value.projectId)
        assertEquals(listOf("Blog"), gateway.createdNames)
    }

    @Test
    fun a_failed_create_shows_failed_and_creates_nothing() {
        gateway.createResult = CreateProjectResult.Unavailable
        val vm = viewModel()
        vm.onNameChange("Blog")
        vm.create()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(NewProjectPhase.Failed, vm.state.value.phase)
        assertNull(vm.state.value.projectId)
        assertTrue(gateway.projects.isEmpty())
    }

    @Test
    fun try_again_after_a_failure_can_succeed() {
        gateway.createResult = CreateProjectResult.Unavailable
        val vm = viewModel()
        vm.onNameChange("Blog")
        vm.create()
        dispatcher.scheduler.advanceUntilIdle()
        gateway.createResult = null
        vm.create()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(NewProjectPhase.Created, vm.state.value.phase)
    }

    @Test
    fun an_exception_from_the_gateway_shows_failed() {
        gateway.failNext = IllegalStateException("database locked")
        val vm = viewModel()
        vm.onNameChange("Blog")
        vm.create()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(NewProjectPhase.Failed, vm.state.value.phase)
    }

    @Test
    fun an_invalid_name_does_not_call_the_gateway() {
        val vm = viewModel()
        vm.onNameChange("")
        vm.create()
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(gateway.createdNames.isEmpty())
        assertEquals(NewProjectPhase.Editing, vm.state.value.phase)
    }

    @Test
    fun typing_is_ignored_while_the_project_is_being_created() {
        val vm = viewModel()
        vm.onNameChange("Blog")
        vm.create()
        vm.onNameChange("Other")
        assertEquals("Blog", vm.state.value.name)
    }
}
