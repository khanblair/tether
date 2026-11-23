package app.tether.settings.github

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
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
class CloneViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeGithubGateway()
    private lateinit var vm: CloneViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun createViewModel(): CloneViewModel {
        vm = CloneViewModel(gateway, io = dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    private fun signedIn() {
        gateway.status = GithubStatus(true, "octo", null, null, listOf("repo"))
    }

    @Test
    fun a_signed_out_user_loads_no_repositories() {
        createViewModel()
        assertEquals(false, vm.state.value.signedIn)
        assertTrue(gateway.pagesRequested.isEmpty())
    }

    @Test
    fun a_signed_in_user_loads_the_first_page() {
        signedIn()
        createViewModel()
        assertEquals(true, vm.state.value.signedIn)
        assertEquals(listOf(1), gateway.pagesRequested)
        assertEquals(GITHUB_PAGE_SIZE, vm.state.value.repos.size)
        assertEquals(1, vm.state.value.repoPage)
    }

    @Test
    fun a_short_page_means_there_is_no_more_to_load() {
        signedIn()
        gateway.repoSource = { listOf(GithubRepo("https://github.com/o/r.git", "main", false, 1)) }
        createViewModel()
        assertFalse(vm.state.value.reposHaveMore)
        vm.loadMore()
        advanceAll()
        assertEquals(listOf(1), gateway.pagesRequested)
    }

    @Test
    fun a_full_page_lets_the_user_load_the_next_page() {
        signedIn()
        createViewModel()
        assertTrue(vm.state.value.reposHaveMore)
        vm.loadMore()
        advanceAll()
        assertEquals(listOf(1, 2), gateway.pagesRequested)
        assertEquals(2, vm.state.value.repoPage)
    }

    @Test
    fun a_list_failure_keeps_the_page_so_try_again_asks_for_it_again() {
        signedIn()
        gateway.listFailure = GithubFailure.Network
        createViewModel()
        assertEquals(GithubFailure.Network, vm.state.value.reposFailure)
        gateway.listFailure = null
        vm.loadMore()
        advanceAll()
        assertNull(vm.state.value.reposFailure)
        assertEquals(listOf(1, 1), gateway.pagesRequested)
    }

    @Test
    fun picking_a_repository_fills_the_url_and_the_project_name() {
        createViewModel()
        vm.pickRepo(GithubRepo("https://github.com/octo/tether.git", "main", false, 1))
        assertEquals("https://github.com/octo/tether.git", vm.state.value.url)
        assertEquals("tether", vm.state.value.projectName)
    }

    @Test
    fun the_project_name_follows_the_url_until_the_user_edits_it() {
        createViewModel()
        vm.onUrlChange("https://github.com/octo/alpha.git")
        assertEquals("alpha", vm.state.value.projectName)
        vm.onProjectNameChange("my app")
        vm.onUrlChange("https://github.com/octo/beta.git")
        assertEquals("my app", vm.state.value.projectName)
    }

    @Test
    fun a_clone_reports_progress_then_returns_the_project_id() {
        createViewModel()
        var seen: CloneProgress? = null
        gateway.cloneBehavior = { _, _, _, onProgress ->
            onProgress(CloneProgress(ClonePhaseStep.Resolving, 0, null))
            onProgress(CloneProgress(ClonePhaseStep.Receiving, 4_000, 8_000))
            seen = vm.state.value.progress
            "project-9"
        }
        vm.onUrlChange("https://github.com/octo/tether.git")
        vm.startClone()
        assertTrue(vm.state.value.cloning)
        advanceAll()
        assertEquals(CloneProgress(ClonePhaseStep.Receiving, 4_000, 8_000), seen)
        assertFalse(vm.state.value.cloning)
        assertNull(vm.state.value.progress)
        assertEquals("project-9", vm.state.value.clonedProjectId)
        vm.clearCloned()
        assertNull(vm.state.value.clonedProjectId)
    }

    @Test
    fun the_clone_sends_the_trimmed_url_the_name_and_the_history() {
        createViewModel()
        vm.onUrlChange("  https://github.com/octo/tether.git ")
        vm.onProjectNameChange("   ")
        vm.onHistoryChange(CloneHistory.Shallow)
        vm.startClone()
        advanceAll()
        assertEquals(
            listOf(Triple("https://github.com/octo/tether.git", null, CloneHistory.Shallow)),
            gateway.clonesStarted,
        )
    }

    @Test
    fun cancel_stops_the_clone_and_shows_canceled_not_failed() {
        createViewModel()
        gateway.cloneBehavior =
            { _, _, _, _ -> throw GithubFailureException(GithubFailure.RepositoryFailed) }
        vm.onUrlChange("https://github.com/octo/tether.git")
        vm.startClone()
        vm.cancelClone()
        vm.cancelClone()
        advanceAll()
        assertEquals(1, gateway.cancelCalls)
        assertTrue(vm.state.value.canceled)
        assertNull(vm.state.value.failure)
        assertFalse(vm.state.value.cloning)
    }

    @Test
    fun cancel_does_nothing_when_no_clone_runs() {
        createViewModel()
        vm.cancelClone()
        advanceAll()
        assertEquals(0, gateway.cancelCalls)
    }

    @Test
    fun a_failed_clone_can_be_tried_again() {
        createViewModel()
        var attempts = 0
        gateway.cloneBehavior = { _, _, _, _ ->
            attempts++
            if (attempts == 1) throw GithubFailureException(GithubFailure.RepositoryFailed)
            "project-2"
        }
        vm.onUrlChange("https://github.com/octo/tether.git")
        vm.startClone()
        advanceAll()
        assertEquals(GithubFailure.RepositoryFailed, vm.state.value.failure)
        assertNull(vm.state.value.clonedProjectId)
        vm.startClone()
        advanceAll()
        assertNull(vm.state.value.failure)
        assertEquals("project-2", vm.state.value.clonedProjectId)
        assertEquals(2, attempts)
    }

    @Test
    fun a_blank_url_does_not_start_a_clone() {
        createViewModel()
        vm.onUrlChange("   ")
        vm.startClone()
        advanceAll()
        assertTrue(gateway.clonesStarted.isEmpty())
    }

    @Test
    fun the_search_query_is_kept_for_the_list() {
        createViewModel()
        vm.onQueryChange("tether")
        assertEquals("tether", vm.state.value.query)
    }

    private fun advanceAll() {
        dispatcher.scheduler.advanceUntilIdle()
    }
}
