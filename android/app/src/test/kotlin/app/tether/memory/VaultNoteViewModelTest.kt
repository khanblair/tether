package app.tether.memory

import app.tether.memory.BodyPart.BodyLink
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
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class VaultNoteViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val source = "---\ndescription: Auth\n---\nSee [[Token rotation]] and [[Commit style]]."
    private val outgoing =
        listOf(
            VaultLink("notes/auth.md", "Token rotation", null, null, resolved = false),
            VaultLink("notes/auth.md", "Commit style", null, null, resolved = true),
        )
    private val incoming = listOf(VaultLink("index.md", "notes/auth", null, null, resolved = true))

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun gateway(vararg extra: VaultNote): FakeVaultGateway {
        val notes = mutableMapOf("notes/auth.md" to note("notes/auth.md", source))
        extra.forEach { notes[it.path] = it }
        return FakeVaultGateway(
            notes,
            links =
                mapOf(
                    "notes/auth.md" to VaultLinks(outgoing, incoming),
                ),
        )
    }

    @Test
    fun the_body_shows_its_links_marked_resolved_or_missing() =
        runTest(dispatcher) {
            val vm = VaultNoteViewModel(gateway(), "notes/auth.md", dispatcher)
            advanceUntilIdle()
            val parts = vm.state.value.parts
            assertEquals(
                listOf(
                    "See ",
                    "Token rotation",
                    " and ",
                    "Commit style",
                    ".",
                ),
                parts.map { if (it is BodyLink) it.label else (it as BodyPart.BodyText).text },
            )
            assertEquals(
                listOf(false, true),
                parts.filterIsInstance<BodyLink>().map { it.resolved },
            )
        }

    @Test
    fun the_backlinks_come_from_the_core() =
        runTest(dispatcher) {
            val vm = VaultNoteViewModel(gateway(), "notes/auth.md", dispatcher)
            advanceUntilIdle()
            assertEquals(
                listOf("index.md"),
                vm.state.value.backlinks
                    .map { it.from },
            )
            assertFalse(vm.state.value.loading)
        }

    @Test
    fun a_resolved_link_opens_the_note_at_the_vault_root_first() =
        runTest(dispatcher) {
            val vm =
                VaultNoteViewModel(
                    gateway(note("Commit style.md", "root")),
                    "notes/auth.md",
                    dispatcher,
                )
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(
                vm.state.value.parts
                    .filterIsInstance<BodyLink>()
                    .last(),
            ) { opened += it }
            advanceUntilIdle()
            assertEquals(listOf("Commit style.md"), opened)
            assertFalse(vm.state.value.linkFailed)
        }

    @Test
    fun a_resolved_link_falls_back_to_the_note_folder() =
        runTest(dispatcher) {
            val vm =
                VaultNoteViewModel(
                    gateway(note("notes/Commit style.md", "folder")),
                    "notes/auth.md",
                    dispatcher,
                )
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(
                vm.state.value.parts
                    .filterIsInstance<BodyLink>()
                    .last(),
            ) { opened += it }
            advanceUntilIdle()
            assertEquals(listOf("notes/Commit style.md"), opened)
        }

    @Test
    fun a_resolved_link_whose_note_cannot_be_read_says_so_and_opens_nothing() =
        runTest(dispatcher) {
            val vm = VaultNoteViewModel(gateway(), "notes/auth.md", dispatcher)
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(
                vm.state.value.parts
                    .filterIsInstance<BodyLink>()
                    .last(),
            ) { opened += it }
            advanceUntilIdle()
            assertEquals(emptyList<String>(), opened)
            assertTrue(vm.state.value.linkFailed)
        }

    @Test
    fun an_unresolved_link_is_not_tappable() =
        runTest(dispatcher) {
            val vm =
                VaultNoteViewModel(
                    gateway(note("Token rotation.md", "present")),
                    "notes/auth.md",
                    dispatcher,
                )
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(
                vm.state.value.parts
                    .filterIsInstance<BodyLink>()
                    .first(),
            ) { opened += it }
            advanceUntilIdle()
            assertEquals(emptyList<String>(), opened)
            assertFalse(vm.state.value.linkFailed)
        }

    @Test
    fun a_missing_note_shows_not_found() =
        runTest(dispatcher) {
            val vm = VaultNoteViewModel(FakeVaultGateway(), "gone.md", dispatcher)
            advanceUntilIdle()
            assertEquals(AreaProblem.NotFound, vm.state.value.problem)
            assertFalse(vm.state.value.loading)
        }
}
