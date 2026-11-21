package app.tether.memory

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
class MemoryViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(gateway: FakeMemoryGateway) = MemoryViewModel(gateway, dispatcher)

    @Test
    fun loads_the_entries_and_the_character_count_of_memory() =
        runTest(dispatcher) {
            val vm =
                viewModel(
                    FakeMemoryGateway(
                        mapOf(
                            MemoryFile.Memory to "Use pytest\n§\nPython 3.12",
                        ),
                    ),
                )
            advanceUntilIdle()
            val state = vm.state.value
            assertFalse(state.loading)
            assertEquals(listOf("Use pytest", "Python 3.12"), state.entries)
            assertEquals(24, state.used)
            assertEquals(2200, state.cap)
        }

    @Test
    fun add_stores_the_trimmed_draft_and_clears_the_field() =
        runTest(dispatcher) {
            val vm = viewModel(FakeMemoryGateway())
            advanceUntilIdle()
            vm.onDraftChange("  Run tests with pytest -q  ")
            vm.add()
            advanceUntilIdle()
            assertEquals(listOf("Run tests with pytest -q"), vm.state.value.entries)
            assertEquals("", vm.state.value.draft)
        }

    @Test
    fun a_blank_draft_adds_nothing() =
        runTest(dispatcher) {
            val gateway = FakeMemoryGateway(mapOf(MemoryFile.Memory to "Kept"))
            val vm = viewModel(gateway)
            advanceUntilIdle()
            vm.onDraftChange("   ")
            vm.add()
            advanceUntilIdle()
            assertEquals("Kept", gateway.files[MemoryFile.Memory])
            assertEquals(listOf("Kept"), vm.state.value.entries)
        }

    @Test
    fun an_add_over_the_cap_keeps_the_draft_and_shows_the_reason() =
        runTest(dispatcher) {
            val full = "x".repeat(2190)
            val vm = viewModel(FakeMemoryGateway(mapOf(MemoryFile.Memory to full)))
            advanceUntilIdle()
            vm.onDraftChange("y".repeat(20))
            vm.add()
            advanceUntilIdle()
            assertEquals(AreaProblem.OverCap, vm.state.value.problem)
            assertEquals("y".repeat(20), vm.state.value.draft)
            assertEquals(listOf(full), vm.state.value.entries)
        }

    @Test
    fun edit_replaces_the_entry_and_closes_the_field() =
        runTest(dispatcher) {
            val vm = viewModel(FakeMemoryGateway(mapOf(MemoryFile.Memory to "One\n§\nPython 3.12")))
            advanceUntilIdle()
            vm.startEdit(1)
            assertEquals("Python 3.12", vm.state.value.editText)
            vm.onEditTextChange("Python 3.13")
            vm.saveEdit()
            advanceUntilIdle()
            assertEquals(listOf("One", "Python 3.13"), vm.state.value.entries)
            assertNull(vm.state.value.editing)
        }

    @Test
    fun a_blank_edit_is_refused_and_nothing_is_written() =
        runTest(dispatcher) {
            val gateway = FakeMemoryGateway(mapOf(MemoryFile.Memory to "Keep me"))
            val vm = viewModel(gateway)
            advanceUntilIdle()
            vm.startEdit(0)
            vm.onEditTextChange("  ")
            vm.saveEdit()
            advanceUntilIdle()
            assertEquals(AreaProblem.InvalidEntry, vm.state.value.problem)
            assertEquals(0, vm.state.value.editing)
            assertEquals("Keep me", gateway.files[MemoryFile.Memory])
        }

    @Test
    fun saving_an_unchanged_entry_only_closes_the_field() =
        runTest(dispatcher) {
            val gateway = FakeMemoryGateway(mapOf(MemoryFile.Memory to "Same"))
            val vm = viewModel(gateway)
            advanceUntilIdle()
            vm.startEdit(0)
            vm.saveEdit()
            advanceUntilIdle()
            assertNull(vm.state.value.editing)
            assertEquals("Same", gateway.files[MemoryFile.Memory])
        }

    @Test
    fun remove_drops_the_entry() =
        runTest(dispatcher) {
            val vm = viewModel(FakeMemoryGateway(mapOf(MemoryFile.Memory to "One\n§\nTwo")))
            advanceUntilIdle()
            vm.remove(0)
            advanceUntilIdle()
            assertEquals(listOf("Two"), vm.state.value.entries)
        }

    @Test
    fun selecting_the_user_file_loads_it_with_its_own_cap() =
        runTest(dispatcher) {
            val vm = viewModel(FakeMemoryGateway(mapOf(MemoryFile.User to "Prefers short answers")))
            advanceUntilIdle()
            vm.select(MemoryFile.User)
            advanceUntilIdle()
            assertEquals(MemoryFile.User, vm.state.value.file)
            assertEquals(listOf("Prefers short answers"), vm.state.value.entries)
            assertEquals(1375, vm.state.value.cap)
        }

    @Test
    fun a_failed_read_marks_the_file_unreadable() =
        runTest(dispatcher) {
            val gateway = FakeMemoryGateway()
            gateway.readFailure = AreaFailure(AreaProblem.Unavailable)
            val vm = viewModel(gateway)
            advanceUntilIdle()
            assertTrue(vm.state.value.readFailed)
            assertFalse(vm.state.value.loading)
            assertEquals(AreaProblem.Unavailable, vm.state.value.problem)
        }
}
