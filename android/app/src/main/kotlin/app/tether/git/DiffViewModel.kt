package app.tether.git

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * State holder for the diff screen. [path] limits the diff to one file, or is null for all
 * changes. Switching the staged side reads the diff again for that side. A newer read cancels an
 * older one still running.
 */
class DiffViewModel(
    private val projectId: String,
    private val path: String?,
    initialStaged: Boolean,
    private val gateway: GitGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(DiffUiState(path = path, staged = initialStaged))

    /** The screen state. */
    val state: StateFlow<DiffUiState> = mutableState.asStateFlow()

    private var loadJob: Job? = null

    init {
        load()
    }

    /** The user picked a side. Picking the side already shown does nothing. */
    fun setStaged(staged: Boolean) {
        if (staged == mutableState.value.staged) return
        mutableState.update { it.copy(staged = staged) }
        load()
    }

    /** The user turned line wrapping on or off. No read is needed. */
    fun toggleWrap() {
        mutableState.update { it.copy(wrap = !it.wrap) }
    }

    /** Reads the diff again. Also used by Try again after a failure. */
    fun retry() {
        load()
    }

    private fun load() {
        val request = DiffRequest(staged = mutableState.value.staged, path = path)
        mutableState.update { it.copy(content = Loadable.Loading) }
        loadJob?.cancel()
        loadJob =
            viewModelScope.launch {
                val outcome = callCore(io) { readDiff(request) }
                val result: Loadable<DiffContent> =
                    outcome.fold(
                        onSuccess = { Loadable.Ready(it) },
                        onFailure = { Loadable.Failed(it.gitFailure()) },
                    )
                mutableState.update { it.copy(content = result) }
            }
    }

    private fun readDiff(request: DiffRequest): DiffContent {
        val text = gateway.diff(projectId, request)
        return DiffContent(
            files = UnifiedDiffParser.parse(text),
            truncated = UnifiedDiffParser.isTruncated(text),
        )
    }
}
