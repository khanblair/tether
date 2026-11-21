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
class VaultEditViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val original = "---\ndescription: Auth\n---\n# Auth"

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun a_new_note_is_written_at_the_typed_path() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway()
            val vm = VaultEditViewModel(gateway, null, dispatcher)
            vm.onPathChange("notes/new.md")
            vm.onSourceChange("---\ndescription: New\n---\nText")
            vm.save()
            advanceUntilIdle()
            assertEquals(listOf("write:notes/new.md"), gateway.writes)
            assertEquals("notes/new.md", vm.state.value.savedPath)
        }

    @Test
    fun a_bad_path_is_refused_before_the_core_is_called() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway()
            val vm = VaultEditViewModel(gateway, null, dispatcher)
            vm.onPathChange("new")
            vm.onSourceChange("text")
            vm.save()
            advanceUntilIdle()
            assertEquals(AreaProblem.InvalidPath, vm.state.value.pathProblem)
            assertEquals(emptyList<String>(), gateway.writes)
            assertNull(vm.state.value.savedPath)
        }

    @Test
    fun a_blank_text_is_not_sent() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway()
            val vm = VaultEditViewModel(gateway, null, dispatcher)
            vm.onPathChange("notes/new.md")
            vm.onSourceChange("   ")
            vm.save()
            advanceUntilIdle()
            assertEquals(emptyList<String>(), gateway.writes)
        }

    @Test
    fun an_existing_note_loads_its_text_and_replace_writes_the_whole_note() =
        runTest(dispatcher) {
            val gateway =
                FakeVaultGateway(
                    mutableMapOf(
                        "notes/auth.md" to note("notes/auth.md", original),
                    ),
                )
            val vm = VaultEditViewModel(gateway, "notes/auth.md", dispatcher)
            advanceUntilIdle()
            assertEquals(original, vm.state.value.source)
            assertTrue(vm.state.value.existing)
            assertFalse(vm.state.value.loading)
            vm.onSourceChange("---\ndescription: Auth\n---\n# Auth v2")
            vm.save()
            advanceUntilIdle()
            assertEquals(listOf("write:notes/auth.md"), gateway.writes)
        }

    @Test
    fun append_sends_only_the_typed_text_and_adds_it_to_the_end() =
        runTest(dispatcher) {
            val gateway =
                FakeVaultGateway(
                    mutableMapOf(
                        "notes/auth.md" to note("notes/auth.md", original),
                    ),
                )
            val vm = VaultEditViewModel(gateway, "notes/auth.md", dispatcher)
            advanceUntilIdle()
            vm.onModeChange(VaultEditMode.Append)
            assertEquals("", vm.state.value.source)
            vm.onSourceChange("\nMore")
            vm.save()
            advanceUntilIdle()
            assertEquals(listOf("append:notes/auth.md:\nMore"), gateway.writes)
            assertEquals(original + "\nMore", gateway.notes["notes/auth.md"]?.source)
        }

    @Test
    fun switching_back_to_replace_shows_the_note_text_again() =
        runTest(dispatcher) {
            val gateway =
                FakeVaultGateway(
                    mutableMapOf(
                        "notes/auth.md" to note("notes/auth.md", original),
                    ),
                )
            val vm = VaultEditViewModel(gateway, "notes/auth.md", dispatcher)
            advanceUntilIdle()
            vm.onModeChange(VaultEditMode.Append)
            vm.onModeChange(VaultEditMode.Replace)
            assertEquals(original, vm.state.value.source)
        }

    @Test
    fun a_refusal_from_the_core_shows_its_problem_and_saves_nothing() =
        runTest(dispatcher) {
            val gateway = FakeVaultGateway()
            val vm = VaultEditViewModel(gateway, null, dispatcher)
            vm.onPathChange("notes/new.md")
            vm.onSourceChange("no frontmatter")
            gateway.failure = AreaFailure(AreaProblem.InvalidNote)
            vm.save()
            advanceUntilIdle()
            assertEquals(AreaProblem.InvalidNote, vm.state.value.problem)
            assertNull(vm.state.value.savedPath)
            assertFalse(vm.state.value.saving)
        }
}
