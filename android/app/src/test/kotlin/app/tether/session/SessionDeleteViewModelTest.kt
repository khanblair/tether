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
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SessionDeleteViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val fixture = SheetFixture()
    private var deleted = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): SessionDeleteViewModel {
        val vm = SessionDeleteViewModel(fixture.deps, onDeleted = { deleted++ }, io = dispatcher)
        advanceUntilIdle()
        return vm
    }

    @Test
    fun the_name_is_the_session_title() {
        assertEquals("Fix login", SessionDeleteState(title = "Fix login").name("New session"))
    }

    @Test
    fun a_blank_title_shows_the_untitled_name() {
        assertEquals("New session", SessionDeleteState(title = "").name("New session"))
        assertEquals("New session", SessionDeleteState(title = "   ").name("New session"))
    }

    @Test
    fun delete_waits_for_the_title_to_load() {
        assertFalse(SessionDeleteState().canDelete)
        assertTrue(SessionDeleteState(title = "Fix login").canDelete)
        assertFalse(SessionDeleteState(title = "Fix login", deleting = true).canDelete)
    }

    @Test
    fun the_loaded_title_names_the_session() =
        runTest(dispatcher) {
            fixture.settings.session = fixture.settings.session.copy(title = "")
            assertEquals("New session", model().state.value.name("New session"))
        }

    @Test
    fun delete_removes_the_session_and_calls_back() =
        runTest(dispatcher) {
            val vm = model()
            vm.delete()
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.contains("delete"))
            assertEquals(1, deleted)
        }

    @Test
    fun a_failed_delete_stays_open_with_a_failure() =
        runTest(dispatcher) {
            val vm = model()
            fixture.settings.failNext = SessionSettingsException(SessionFailure.BUSY)
            vm.delete()
            advanceUntilIdle()
            assertEquals(SessionFailure.BUSY, vm.state.value.failure)
            assertFalse(vm.state.value.deleting)
            assertEquals(0, deleted)
        }
}
