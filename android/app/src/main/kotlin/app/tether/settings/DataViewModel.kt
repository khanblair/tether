package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.workspace.WorkspaceDataPaths
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * What the data screen shows. [usage] is null until the first read. [cacheFailed] is true when a
 * clear left files behind.
 */
@Immutable
data class DataUiState(
    val usage: StorageUsage? = null,
    val paths: WorkspaceDataPaths? = null,
    val cacheFailed: Boolean = false,
)

/** Reads storage sizes and clears the update cache. Disk work runs on [io]. */
class DataViewModel(
    private val source: StorageSource,
    private val onCacheCleared: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(DataUiState())

    /** The screen state. */
    val state: StateFlow<DataUiState> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads the sizes again. */
    fun refresh() {
        viewModelScope.launch(io) {
            val usage = source.usage()
            mutableState.update { it.copy(usage = usage, paths = source.paths()) }
        }
    }

    /**
     * Deletes the update cache. Tells the user through [onCacheCleared] only when every file went.
     */
    fun clearUpdateCache() {
        viewModelScope.launch(io) {
            val cleared = source.clearUpdateCache()
            mutableState.update { it.copy(cacheFailed = !cleared) }
            if (cleared) onCacheCleared()
            refresh()
        }
    }
}
