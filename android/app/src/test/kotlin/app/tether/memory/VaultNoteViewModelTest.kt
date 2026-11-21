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
    private val source =
        "---\ndescription: Auth\n---\nSee [[Token rotation]], [[Commit style]] and [[Auth]]."
    private val outgoing =
        listOf(
            VaultLink(
                "notes/auth.md",
                "Token rotation",
                null,
                null,
                resolved = false,
                resolvedPath = null,
            ),
            VaultLink(
                "notes/auth.md",
                "Commit style",
                null,
                null,
                resolved = true,
                resolvedPath = "Commit style.md",
            ),
            VaultLink(
                "notes/auth.md",
                "Auth",
                null,
                null,
                resolved = true,
                resolvedPath = "notes/Auth.md",
            ),
        )
    private val incoming =
        listOf(
            VaultLink(
                "index.md",
                "notes/auth",
                null,
                null,
                resolved = true,
                resolvedPath = "notes/auth.md",
            ),
        )

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

    private fun VaultNoteViewModel.links(): List<BodyLink> =
        state.value.parts.filterIsInstance<BodyLink>()

    @Test
    fun the_body_shows_its_links_marked_resolved_or_missing() =
        runTest(dispatcher) {
            val vm = VaultNoteViewModel(gateway(), "notes/auth.md", dispatcher)
            advanceUntilIdle()
            assertEquals(
                listOf(false, true, true),
                vm.links().map { it.resolved },
            )
            assertEquals(
                listOf("Token rotation", "Commit style", "Auth"),
                vm.links().map { it.label },
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
    fun a_resolved_link_opens_the_path_the_core_resolved_it_to() =
        runTest(dispatcher) {
            val vm =
                VaultNoteViewModel(
                    gateway(note("notes/Auth.md", "auth note")),
                    "notes/auth.md",
                    dispatcher,
                )
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(vm.links().last()) { opened += it }
            advanceUntilIdle()
            assertEquals(listOf("notes/Auth.md"), opened)
            assertFalse(vm.state.value.linkFailed)
        }

    @Test
    fun a_resolved_link_whose_note_cannot_be_read_says_so_and_opens_nothing() =
        runTest(dispatcher) {
            val vm = VaultNoteViewModel(gateway(), "notes/auth.md", dispatcher)
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(vm.links().last()) { opened += it }
            advanceUntilIdle()
            assertEquals(emptyList<String>(), opened)
            assertTrue(vm.state.value.linkFailed)
        }

    @Test
    fun a_link_with_no_resolved_path_is_missing_and_not_tappable() =
        runTest(dispatcher) {
            val vm =
                VaultNoteViewModel(
                    gateway(note("Token rotation.md", "present")),
                    "notes/auth.md",
                    dispatcher,
                )
            advanceUntilIdle()
            val opened = mutableListOf<String>()
            vm.openLink(vm.links().first()) { opened += it }
            advanceUntilIdle()
            assertEquals(false, vm.links().first().resolved)
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
