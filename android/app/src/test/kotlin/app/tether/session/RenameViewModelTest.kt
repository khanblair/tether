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
class RenameViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val fixture = SheetFixture()
    private var applied = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): RenameViewModel {
        val vm = RenameViewModel(fixture.deps, onApplied = { applied++ }, io = dispatcher)
        advanceUntilIdle()
        return vm
    }

    @Test
    fun the_field_starts_with_the_current_title() =
        runTest(dispatcher) {
            assertEquals("Fix login", model().state.value.title)
        }

    @Test
    fun a_blank_title_cannot_be_saved() =
        runTest(dispatcher) {
            val vm = model()
            vm.onTitleChange("   ")
            assertFalse(vm.state.value.canSave)
            vm.save()
            advanceUntilIdle()
            assertTrue(fixture.settings.calls.none { it.startsWith("rename") })
            assertEquals(0, applied)
        }

    @Test
    fun the_saved_title_is_trimmed() =
        runTest(dispatcher) {
            val vm = model()
            vm.onTitleChange("  Login fix  ")
            vm.save()
            advanceUntilIdle()
            assertEquals("Login fix", fixture.settings.session.title)
            assertEquals(1, applied)
        }

    @Test
    fun a_failed_rename_keeps_the_sheet_open_with_a_failure() =
        runTest(dispatcher) {
            val vm = model()
            fixture.settings.failNext = SessionSettingsException(SessionFailure.NOT_FOUND)
            vm.onTitleChange("Other")
            vm.save()
            advanceUntilIdle()
            assertEquals(SessionFailure.NOT_FOUND, vm.state.value.failure)
            assertEquals(0, applied)
            assertFalse(vm.state.value.saving)
        }
}
