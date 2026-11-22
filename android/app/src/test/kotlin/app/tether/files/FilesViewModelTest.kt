package app.tether.files

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class FilesViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeFilesGateway()

    private val src = FileEntry("src", "src", isDir = true, sizeBytes = 0L, ignored = false)
    private val readme =
        FileEntry("README.md", "README.md", isDir = false, sizeBytes = 10L, ignored = false)
    private val auth =
        FileEntry("auth.kt", "src/auth.kt", isDir = false, sizeBytes = 5L, ignored = false)
    private val target = FileEntry(".env", ".env", isDir = false, sizeBytes = 3L, ignored = true)

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
        gateway.listings[ROOT_DIR] = DirListing(listOf(src, readme, target), truncated = false)
        gateway.listings["src"] = DirListing(listOf(auth), truncated = false)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(): FilesViewModel {
        val vm = FilesViewModel("blog", gateway, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    @Test
    fun the_root_loads_once_when_the_screen_opens_and_shows_the_project_name() {
        val vm = viewModel()
        val state = vm.state.value
        assertEquals("Blog", state.projectName)
        assertEquals(listOf(ROOT_DIR to false), gateway.listCalls)
        assertTrue(state.cache[ROOT_DIR] is DirState.Loaded)
    }

    @Test
    fun expanding_a_folder_loads_its_children_once_and_collapsing_keeps_them() {
        val vm = viewModel()
        vm.ensureLoaded("src")
        dispatcher.scheduler.advanceUntilIdle()
        vm.ensureLoaded("src")
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf(ROOT_DIR to false, "src" to false), gateway.listCalls)
        assertEquals(
            DirState.Loaded(listOf(auth), truncated = false),
            vm.state.value.cache["src"],
        )
    }

    @Test
    fun a_failed_folder_stays_failed_until_retry_succeeds() {
        gateway.failing += "src"
        val vm = viewModel()
        vm.ensureLoaded("src")
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(DirState.Failed, vm.state.value.cache["src"])

        gateway.failing -= "src"
        vm.retry("src")
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.state.value.cache["src"] is DirState.Loaded)
        assertEquals(2, gateway.listCalls.count { it.first == "src" })
    }

    @Test
    fun toggling_ignored_files_reloads_the_root_and_open_folders_with_the_new_flag() {
        val vm = viewModel()
        vm.ensureLoaded("src")
        dispatcher.scheduler.advanceUntilIdle()
        gateway.listCalls.clear()

        vm.setShowIgnored(true, expanded = setOf("src"))
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(vm.state.value.showIgnored)
        assertEquals(listOf(ROOT_DIR to true, "src" to true), gateway.listCalls)
        assertTrue(vm.state.value.cache[ROOT_DIR] is DirState.Loaded)
    }

    @Test
    fun a_blank_query_makes_no_search_call_and_shows_no_hits() {
        val vm = viewModel()
        vm.setSearchOpen(true)
        vm.onQueryChange("   ")
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(gateway.nameCalls.isEmpty())
        assertNull(vm.state.value.nameHits)
    }

    @Test
    fun name_search_waits_300_ms_after_the_last_keystroke() {
        gateway.nameHits = listOf("src/auth.kt")
        val vm = viewModel()
        vm.setSearchOpen(true)
        vm.onQueryChange("au")
        dispatcher.scheduler.advanceTimeBy(100)
        vm.onQueryChange("aut")
        dispatcher.scheduler.advanceTimeBy(299)
        dispatcher.scheduler.runCurrent()
        assertTrue(gateway.nameCalls.isEmpty())

        dispatcher.scheduler.advanceTimeBy(1)
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf("aut"), gateway.nameCalls)
        assertEquals(listOf("src/auth.kt"), vm.state.value.nameHits)
    }

    @Test
    fun a_failed_name_search_shows_a_problem_and_no_hits() {
        gateway.nameFailure = FilesFailure(FilesProblem.Unavailable)
        val vm = viewModel()
        vm.setSearchOpen(true)
        vm.onQueryChange("auth")
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(vm.state.value.nameProblem)
        assertEquals(emptyList<String>(), vm.state.value.nameHits)
    }

    @Test
    fun text_search_runs_only_when_asked_and_lists_lines_with_their_numbers() {
        gateway.textHits = listOf(FileTextHit("src/auth.kt", line = 12, text = "fun login()"))
        val vm = viewModel()
        vm.setSearchOpen(true)
        vm.onQueryChange("login")
        dispatcher.scheduler.advanceUntilIdle()
        assertTrue(gateway.textCalls.isEmpty())

        vm.searchText()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(listOf("login"), gateway.textCalls)
        assertEquals(
            TextSearch.Hits(listOf(FileTextHit("src/auth.kt", 12, "fun login()"))),
            vm.state.value.textSearch,
        )
    }

    @Test
    fun a_failed_text_search_says_so() {
        gateway.textFailure = FilesFailure(FilesProblem.Unavailable)
        val vm = viewModel()
        vm.setSearchOpen(true)
        vm.onQueryChange("login")
        vm.searchText()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(TextSearch.Failed, vm.state.value.textSearch)
    }

    @Test
    fun closing_search_clears_the_query_and_its_results() {
        gateway.nameHits = listOf("README.md")
        val vm = viewModel()
        vm.setSearchOpen(true)
        vm.onQueryChange("read")
        dispatcher.scheduler.advanceUntilIdle()
        vm.setSearchOpen(false)
        val state = vm.state.value
        assertEquals("", state.query)
        assertNull(state.nameHits)
        assertEquals(TextSearch.Idle, state.textSearch)
        assertEquals(false, state.searchOpen)
    }
}
