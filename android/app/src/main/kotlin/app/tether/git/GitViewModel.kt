package app.tether.git

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * State holder for the Git screen of one project. The Changes tab loads when the screen opens.
 * Each other tab loads the first time it is opened. [refresh] and Try again read the open tab
 * again. Every read runs on [io], and the clock is read there too, so tests control both.
 */
class GitViewModel(
    private val projectId: String,
    private val gateway: GitGateway,
    private val clock: () -> Long = System::currentTimeMillis,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(GitUiState())

    /** The screen state. */
    val state: StateFlow<GitUiState> = mutableState.asStateFlow()

    init {
        load(GitTab.Changes)
    }

    /** The user opened a tab. A tab that has not loaded yet starts its read here. */
    fun selectTab(tab: GitTab) {
        mutableState.update { it.copy(tab = tab) }
        if (loadableOf(tab) == Loadable.Idle) load(tab)
    }

    /** Reads the open tab again. Also used by Try again after a failure. */
    fun refresh() {
        load(mutableState.value.tab)
    }

    private fun loadableOf(tab: GitTab): Loadable<*> =
        when (tab) {
            GitTab.Changes -> mutableState.value.changes
            GitTab.Branches -> mutableState.value.branches
            GitTab.History -> mutableState.value.history
        }

    private fun load(tab: GitTab) {
        when (tab) {
            GitTab.Changes -> read({ readChanges() }) { s, v -> s.copy(changes = v) }
            GitTab.Branches -> read({ readBranches() }) { s, v -> s.copy(branches = v) }
            GitTab.History -> read({ readHistory() }) { s, v -> s.copy(history = v) }
        }
    }

    /** Sets [Loadable.Loading], runs [block] on [io], then stores the result with [store]. */
    private fun <T> read(
        block: () -> T,
        store: (GitUiState, Loadable<T>) -> GitUiState,
    ) {
        mutableState.update { store(it, Loadable.Loading) }
        viewModelScope.launch {
            val outcome = callCore(io, block)
            val result: Loadable<T> =
                outcome.fold(
                    onSuccess = { Loadable.Ready(it) },
                    onFailure = { Loadable.Failed(it.gitFailure()) },
                )
            mutableState.update { store(it, result) }
        }
    }

    private fun readChanges(): ChangesContent {
        val branch = gateway.currentBranch(projectId)
        val entries = gateway.status(projectId)
        return ChangesContent(
            branch = branch,
            staged = entries.filter { it.staged != null },
            unstaged = entries.filter { it.unstaged != null },
        )
    }

    private fun readBranches(): BranchesContent {
        val current = gateway.currentBranch(projectId)
        val listed = gateway.branches(projectId)
        // An unborn branch has no ref yet, so the core does not list it. Show it anyway.
        val rows =
            if (current != null && listed.none { it.name == current }) {
                listOf(GitBranch(current, null)) + listed
            } else {
                listed
            }
        return BranchesContent(current = current, branches = rows)
    }

    private fun readHistory(): HistoryContent {
        val commits = gateway.log(projectId, GIT_HISTORY_LIMIT)
        return HistoryContent(commits = commits, nowMillis = clock())
    }
}
