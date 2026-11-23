package app.tether.git

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class GitViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeGitGateway()
    private var now = NOW_MILLIS
    private lateinit var vm: GitViewModel

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun createViewModel() {
        vm = GitViewModel(PROJECT, gateway, clock = { now }, io = dispatcher)
        dispatcher.scheduler.runCurrent()
    }

    @Test
    fun the_changes_tab_groups_staged_and_not_staged_entries() {
        gateway.entries =
            listOf(
                ChangeEntry("a.txt", ChangeKind.Modified, null),
                ChangeEntry("b.txt", null, ChangeKind.Untracked),
                ChangeEntry("c.txt", ChangeKind.Added, ChangeKind.Modified),
            )

        createViewModel()

        val changes = (vm.state.value.changes as Loadable.Ready<ChangesContent>).value
        assertEquals("main", changes.branch)
        assertEquals(listOf("a.txt", "c.txt"), changes.staged.map { it.path })
        assertEquals(listOf("b.txt", "c.txt"), changes.unstaged.map { it.path })
    }

    @Test
    fun an_unborn_repository_shows_its_branch_with_no_commit() {
        gateway.listed = emptyList()

        createViewModel()
        vm.selectTab(GitTab.Branches)
        dispatcher.scheduler.runCurrent()

        val branches = (vm.state.value.branches as Loadable.Ready<BranchesContent>).value
        assertEquals("main", branches.current)
        assertEquals(listOf(GitBranch("main", null)), branches.branches)
    }

    @Test
    fun a_folder_that_is_not_a_repository_fails_with_that_reason() {
        gateway.statusFailure = GitFailure.NotRepository

        createViewModel()

        assertEquals(
            Loadable.Failed(GitFailure.NotRepository),
            vm.state.value.changes,
        )
    }

    @Test
    fun a_failure_then_try_again_reads_the_tab_again() {
        gateway.statusFailure = GitFailure.RepositoryFailed
        createViewModel()

        gateway.statusFailure = null
        gateway.entries = listOf(ChangeEntry("a.txt", null, ChangeKind.Modified))
        vm.refresh()
        dispatcher.scheduler.runCurrent()

        assertEquals(2, gateway.statusCalls)
        assertTrue(vm.state.value.changes is Loadable.Ready)
    }

    @Test
    fun each_tab_loads_once_and_refresh_reads_the_open_tab() {
        createViewModel()
        assertEquals(1, gateway.statusCalls)

        vm.selectTab(GitTab.Changes)
        dispatcher.scheduler.runCurrent()
        assertEquals(1, gateway.statusCalls)

        vm.selectTab(GitTab.Branches)
        vm.selectTab(GitTab.Branches)
        dispatcher.scheduler.runCurrent()
        assertEquals(1, gateway.branchesCalls)

        vm.refresh()
        dispatcher.scheduler.runCurrent()
        assertEquals(2, gateway.branchesCalls)
        assertEquals(1, gateway.statusCalls)
    }

    @Test
    fun history_asks_for_fifty_commits_and_keeps_the_clock_reading() {
        gateway.commits = listOf(GitCommit("abcdef1234", "Fix login", "Blair", NOW_MILLIS / 1000))
        createViewModel()

        vm.selectTab(GitTab.History)
        dispatcher.scheduler.runCurrent()

        assertEquals(listOf(50), gateway.logLimits)
        val history = (vm.state.value.history as Loadable.Ready<HistoryContent>).value
        assertEquals(NOW_MILLIS, history.nowMillis)
        assertEquals("abcdef1", history.commits.single().shortId)
    }

    @Test
    fun a_history_with_no_commits_is_an_empty_list() {
        createViewModel()

        vm.selectTab(GitTab.History)
        dispatcher.scheduler.runCurrent()

        assertEquals(
            emptyList<GitCommit>(),
            (vm.state.value.history as Loadable.Ready<HistoryContent>).value.commits,
        )
    }

    private companion object {
        const val PROJECT = "project-1"
        const val NOW_MILLIS = 1_700_000_000_000L
    }
}
