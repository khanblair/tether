package app.tether.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * What the Projects screen shows. [projects] is filtered by [query] and sorted by last activity,
 * newest first. [totalCount] counts every project, whatever the query. [now] is the time the
 * relative times are taken from.
 */
@Immutable
data class ProjectsUiState(
    val query: String = "",
    val projects: List<ProjectSummary> = emptyList(),
    val totalCount: Int = 0,
    val now: Long = 0L,
)

/**
 * State holder for the Projects screen. The sources live in memory, so the list loads at once.
 */
class ProjectsViewModel(
    private val source: ProjectsSource,
    private val clock: () -> Long,
) : ViewModel() {
    private val all: List<ProjectSummary> =
        source.projects().sortedByDescending { it.lastActivityAt }
    private val mutableState = MutableStateFlow(build(""))

    /** The list state. */
    val state: StateFlow<ProjectsUiState> = mutableState.asStateFlow()

    /** Filters the list by project name. The search does not open another screen. */
    fun onQueryChange(query: String) {
        mutableState.value = build(query)
    }

    private fun build(query: String): ProjectsUiState {
        val needle = query.trim()
        return ProjectsUiState(
            query = query,
            projects = all.filter { it.name.contains(needle, ignoreCase = true) },
            totalCount = all.size,
            now = clock(),
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
