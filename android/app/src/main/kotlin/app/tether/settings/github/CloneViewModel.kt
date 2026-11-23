package app.tether.settings.github

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * What the clone screen shows. [signedIn] is null until the first status read. [reposHaveMore]
 * is true while the last page was full. [canceled] means the user stopped a clone and nothing
 * was kept. [clonedProjectId] is set once the clone succeeds; the screen then calls `onCloned`.
 */
data class CloneUiState(
    val url: String = "",
    val projectName: String = "",
    val history: CloneHistory = CloneHistory.Auto,
    val signedIn: Boolean? = null,
    val repos: List<GithubRepo> = emptyList(),
    val repoPage: Int = 0,
    val reposHaveMore: Boolean = true,
    val reposLoading: Boolean = false,
    val reposFailure: GithubFailure? = null,
    val query: String = "",
    val cloning: Boolean = false,
    val progress: CloneProgress? = null,
    val failure: GithubFailure? = null,
    val canceled: Boolean = false,
    val clonedProjectId: String? = null,
)

/**
 * State holder for the clone screen. The clone blocks on [io]. Progress arrives from the core on
 * any thread and is written with [MutableStateFlow.update], which is safe across threads. Cancel
 * asks the core to stop; the clone then returns a failure, which shows as canceled, not failed.
 */
class CloneViewModel(
    private val gateway: GithubGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(CloneUiState())

    /** The screen state. */
    val state: StateFlow<CloneUiState> = mutableState.asStateFlow()

    private var cancelRequested = false

    init {
        viewModelScope.launch {
            callCore(io) { gateway.status() }
                .onSuccess { status ->
                    mutableState.update { it.copy(signedIn = status.signedIn) }
                    if (status.signedIn) loadMore()
                }.onFailure {
                    mutableState.update { it.copy(signedIn = false) }
                }
        }
    }

    /** The user typed in the URL field. The name follows the URL until the user edits the name. */
    fun onUrlChange(value: String) {
        mutableState.update { state ->
            val followName =
                state.projectName.isEmpty() || state.projectName == defaultProjectName(state.url)
            state.copy(
                url = value,
                projectName = if (followName) defaultProjectName(value) else state.projectName,
                failure = null,
                canceled = false,
            )
        }
    }

    /** The user tapped a repository row. It fills the URL and the project name. */
    fun pickRepo(repo: GithubRepo) {
        onUrlChange(repo.cloneUrl)
    }

    /** The user typed in the project name field. */
    fun onProjectNameChange(value: String) {
        mutableState.update { it.copy(projectName = value) }
    }

    /** The user chose a history depth. */
    fun onHistoryChange(history: CloneHistory) {
        mutableState.update { it.copy(history = history) }
    }

    /** The user typed in the repository search field. */
    fun onQueryChange(value: String) {
        mutableState.update { it.copy(query = value) }
    }

    /** The user asked for the next page of repositories. */
    fun loadMore() {
        val current = mutableState.value
        if (current.reposLoading || !current.reposHaveMore) return
        val page = current.repoPage + 1
        mutableState.update { it.copy(reposLoading = true, reposFailure = null) }
        viewModelScope.launch {
            callCore(io) { gateway.listRepos(page) }
                .onSuccess { repos ->
                    mutableState.update {
                        it.copy(
                            repos = it.repos + repos,
                            repoPage = page,
                            reposHaveMore = repos.size >= GITHUB_PAGE_SIZE,
                            reposLoading = false,
                        )
                    }
                }.onFailure { error ->
                    mutableState.update {
                        it.copy(reposLoading = false, reposFailure = error.githubFailure())
                    }
                }
        }
    }

    /** The user pressed Clone, or Try again after a failure. */
    fun startClone() {
        val current = mutableState.value
        val url = current.url.trim()
        if (current.cloning || url.isEmpty()) return
        val name = current.projectName.trim().ifEmpty { null }
        val history = current.history
        cancelRequested = false
        mutableState.update {
            it.copy(cloning = true, progress = null, failure = null, canceled = false)
        }
        viewModelScope.launch {
            callCore(io) {
                gateway.cloneRepository(url, name, history) { report ->
                    mutableState.update { it.copy(progress = report) }
                }
            }.onSuccess { projectId ->
                mutableState.update {
                    it.copy(cloning = false, progress = null, clonedProjectId = projectId)
                }
            }.onFailure { error ->
                mutableState.update {
                    if (cancelRequested) {
                        it.copy(cloning = false, progress = null, canceled = true)
                    } else {
                        it.copy(
                            cloning = false,
                            progress = null,
                            failure = error.githubFailure(),
                        )
                    }
                }
            }
        }
    }

    /** The user pressed Cancel. Does nothing when no clone runs or cancel was already asked. */
    fun cancelClone() {
        if (!mutableState.value.cloning || cancelRequested) return
        cancelRequested = true
        viewModelScope.launch {
            callCore(io) { gateway.cancelClone() }
        }
    }

    /** The user acknowledged the success hand-off, so the screen does not repeat it. */
    fun clearCloned() {
        mutableState.update { it.copy(clonedProjectId = null) }
    }

    companion object {
        /** The clone view model over [gateway], for the New project flow's Clone choice. */
        fun factory(gateway: GithubGateway) =
            viewModelFactory { initializer { CloneViewModel(gateway) } }
    }
}
