package app.tether.session

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

private const val SEARCH_DEBOUNCE_MS = 300L
private const val SEARCH_LIMIT = 50

/** Which sessions a search covers. */
enum class SearchScope { THIS_PROJECT, ALL_PROJECTS }

/** The search screen's state. [searched] is true once a search for [query] has finished. */
data class SearchState(
    val query: String = "",
    val scope: SearchScope = SearchScope.ALL_PROJECTS,
    val loading: Boolean = false,
    val searched: Boolean = false,
    val hits: List<SearchHit> = emptyList(),
    val failure: SessionFailure? = null,
)

/**
 * Searches messages across all projects, or session titles in [projectId] when there is one.
 * Typing waits 300 ms before it searches. A blank query never reaches the core.
 */
class SearchViewModel(
    private val settings: SessionSettingsGateway,
    private val projectId: String?,
    initialQuery: String,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(
            SearchState(
                query = initialQuery,
                scope =
                    if (projectId == null) SearchScope.ALL_PROJECTS else SearchScope.THIS_PROJECT,
            ),
        )
    private var pending: Job? = null

    /** The screen state. */
    val state: StateFlow<SearchState> = mutableState.asStateFlow()

    init {
        if (initialQuery.isNotBlank()) runNow()
    }

    /** The user typed in the search field. The search starts after the debounce delay. */
    fun onQueryChange(text: String) {
        mutableState.update { it.copy(query = text) }
        pending?.cancel()
        if (text.isBlank()) {
            mutableState.update { SearchState(query = text, scope = it.scope) }
            return
        }
        pending =
            viewModelScope.launch {
                delay(SEARCH_DEBOUNCE_MS)
                search()
            }
    }

    /** The user picked a scope. A non-blank query runs again at once. */
    fun setScope(scope: SearchScope) {
        if (projectId == null) return
        mutableState.update { it.copy(scope = scope) }
        if (mutableState.value.query.isNotBlank()) runNow()
    }

    private fun runNow() {
        pending?.cancel()
        pending = viewModelScope.launch { search() }
    }

    private suspend fun search() {
        val query = mutableState.value.query.trim()
        if (query.isEmpty()) return
        val scope = mutableState.value.scope
        mutableState.update { it.copy(loading = true, failure = null) }
        sessionCall(io) {
            when (scope) {
                SearchScope.THIS_PROJECT -> settings.searchSessions(checkNotNull(projectId), query)
                SearchScope.ALL_PROJECTS -> settings.searchTranscripts(query, SEARCH_LIMIT)
            }
        }.onSuccess { hits ->
            mutableState.update { it.copy(loading = false, searched = true, hits = hits) }
        }.onFailure { failure ->
            mutableState.update {
                it.copy(
                    loading = false,
                    searched = true,
                    hits = emptyList(),
                    failure = failure.sessionFailure(),
                )
            }
        }
    }

    companion object {
        /** The search view model. [projectId] is null when no project is in view. */
        fun factory(
            settings: SessionSettingsGateway,
            projectId: String?,
            initialQuery: String,
        ) = viewModelFactory {
            initializer { SearchViewModel(settings, projectId, initialQuery) }
        }
    }
}
