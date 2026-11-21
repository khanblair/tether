package app.tether.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * What the Projects screen shows. [projects] is filtered by [query] and sorted by last activity,
 * newest first. [totalCount] counts every project, whatever the query. [now] is the time the
 * relative times are taken from. [loading] stays true until the first read finishes.
 */
@Immutable
data class ProjectsUiState(
    val query: String = "",
    val projects: List<ProjectSummary> = emptyList(),
    val totalCount: Int = 0,
    val now: Long = 0L,
    val loading: Boolean = true,
)

/**
 * State holder for the Projects screen. The source reads the core, which blocks, so it is read on
 * [io]. [refresh] reads it again when the screen resumes.
 */
class ProjectsViewModel(
    private val source: ProjectsSource,
    private val clock: () -> Long,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val loaded = MutableStateFlow<List<ProjectSummary>?>(null)
    private val mutableState = MutableStateFlow(build(null, ""))

    /** The list state. */
    val state: StateFlow<ProjectsUiState> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads the projects again. The list stays on screen while the read runs. */
    fun refresh() {
        viewModelScope.launch(io) {
            val projects = source.projects().sortedByDescending { it.lastActivityAt }
            loaded.value = projects
            mutableState.value = build(projects, mutableState.value.query)
        }
    }

    /** Filters the list by project name. The search does not open another screen. */
    fun onQueryChange(text: String) {
        mutableState.value = build(loaded.value, text)
    }

    private fun build(
        all: List<ProjectSummary>?,
        typed: String,
    ): ProjectsUiState {
        val needle = typed.trim()
        val projects = all.orEmpty()
        return ProjectsUiState(
            query = typed,
            projects = projects.filter { it.name.contains(needle, ignoreCase = true) },
            totalCount = projects.size,
            now = clock(),
            loading = all == null,
        )
    }

    companion object {
        /** Builds the list model from [source]. [clock] gives the time in milliseconds. */
        fun factory(
            source: ProjectsSource,
            clock: () -> Long = { System.currentTimeMillis() },
        ) = viewModelFactory {
            initializer { ProjectsViewModel(source, clock) }
        }
    }
}
