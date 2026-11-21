package app.tether.memory

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * What the Vault list shows. With a blank [query] it shows the two standard notes, the catalog
 * and the log, when they exist. With a query it shows the search hits in the core's order.
 */
@Immutable
data class VaultListUiState(
    val query: String = "",
    val notes: List<VaultNote> = emptyList(),
    val loading: Boolean = true,
    val problem: AreaProblem? = null,
)

/** State holder for the Vault list. Each read and search runs on [io]. */
class VaultListViewModel(
    private val gateway: VaultGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(VaultListUiState())

    /** The list state. */
    val state: StateFlow<VaultListUiState> = mutableState.asStateFlow()

    init {
        showStandardNotes()
    }

    /** Searches the vault for [text]. A blank text shows the standard notes again. */
    fun onQueryChange(text: String) {
        mutableState.value = mutableState.value.copy(query = text, problem = null)
        val needle = text.trim()
        if (needle.isEmpty()) {
            showStandardNotes()
            return
        }
        viewModelScope.launch {
            val result = callCore(io) { gateway.search(needle, SEARCH_LIMIT) }
            if (mutableState.value.query.trim() == needle) publish(result)
        }
    }

    private fun showStandardNotes() {
        viewModelScope.launch { publish(callCore(io) { standardNotes() }) }
    }

    private fun standardNotes(): List<VaultNote> =
        STANDARD_NOTES.mapNotNull { path ->
            try {
                gateway.read(path)
            } catch (missing: AreaFailure) {
                if (missing.problem == AreaProblem.NotFound) null else throw missing
            }
        }

    /** Shows a finished read: a failure sets the problem, a success clears it. */
    private fun publish(result: Result<List<VaultNote>>) {
        val current = mutableState.value
        mutableState.value =
            result.fold(
                onSuccess = { current.copy(notes = it, loading = false, problem = null) },
                onFailure = {
                    current.copy(
                        notes = emptyList(),
                        loading = false,
                        problem = it.areaProblem(),
                    )
                },
            )
    }

    private companion object {
        const val SEARCH_LIMIT = 50
        val STANDARD_NOTES = listOf("index.md", "log.md")
    }
}
