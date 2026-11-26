package app.tether.session

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
class SessionMenuViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val fixture = SheetFixture()
    private var archived = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): SessionMenuViewModel {
        val vm =
            SessionMenuViewModel(fixture.deps, onArchived = { archived++ }, io = dispatcher)
        advanceUntilIdle()
        return vm
    }

    private fun checkpoints(count: Int) =
        List(count) {
            CheckpointItem(id = "cp-$it", createdAtMs = it.toLong(), paths = emptyList())
        }

    @Test
    fun the_count_is_the_number_of_checkpoints() =
        runTest(dispatcher) {
            fixture.settings.checkpointList = checkpoints(3)
            assertEquals(3, model().state.value.checkpoints)
        }

    @Test
    fun a_session_with_no_checkpoints_has_a_count_of_zero() =
        runTest(dispatcher) {
            assertEquals(0, model().state.value.checkpoints)
        }

    @Test
    fun archive_and_delete_are_off_while_a_turn_runs() {
        val state = SessionMenuState(checkpoints = 1)
        assertFalse(state.canArchive(running = true))
        assertTrue(state.canArchive(running = false))
        assertFalse(state.copy(archiving = true).canArchive(running = false))
    }

    @Test
    fun archive_does_nothing_while_a_turn_runs() =
        runTest(dispatcher) {
            val vm = model()
            fixture.running = true
            vm.archive()
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("archive") })
            assertEquals(0, archived)
        }

    @Test
    fun archive_hides_the_session_and_calls_back() =
        runTest(dispatcher) {
            val vm = model()
            vm.archive()
            advanceUntilIdle()
            val archiveCalls = fixture.settings.calls.filter { it.startsWith("archive") }
            assertEquals(listOf("archive:true"), archiveCalls)
            assertEquals(1, archived)
            assertNull(vm.state.value.failure)
        }

    @Test
    fun a_busy_archive_stays_open_with_a_failure() =
        runTest(dispatcher) {
            val vm = model()
            fixture.settings.failNext = SessionSettingsException(SessionFailure.BUSY)
            vm.archive()
            advanceUntilIdle()
            assertEquals(SessionFailure.BUSY, vm.state.value.failure)
            assertFalse(vm.state.value.archiving)
            assertEquals(0, archived)
        }

    @Test
    fun undo_restores_the_session_through_the_session_gateway() =
        runTest(dispatcher) {
            val gateway = FakeSessionGateway()
            val result = restoreArchivedSession(gateway, "session-1")
            assertTrue(result.isSuccess)
            assertEquals(listOf("session-1" to false), gateway.archiveCalls)
        }
}
