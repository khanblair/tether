package app.tether.files

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The text search under the field. It runs only when the user asks for it. */
sealed interface TextSearch {
    /** No text search has run for the current query. */
    data object Idle : TextSearch

    /** The text search is running. */
    data object Loading : TextSearch

    /** The lines that matched, in the core's order. */
    data class Hits(
        val hits: List<FileTextHit>,
    ) : TextSearch

    /** The text search failed. */
    data object Failed : TextSearch
}

/**
 * What the file browser shows. [cache] holds each folder's listing; the screen decides which
 * folders are expanded. [nameHits] is null before the first name search for the query.
 */
@Immutable
data class FilesUiState(
    val projectName: String? = null,
    val cache: Map<String, DirState> = emptyMap(),
    val showIgnored: Boolean = false,
    val searchOpen: Boolean = false,
    val query: String = "",
    val nameHits: List<String>? = null,
    val nameProblem: Boolean = false,
    val textSearch: TextSearch = TextSearch.Idle,
)

/**
 * State holder for the file browser. Listings and searches run on [io]. A folder loads its
 * children once, when it is first needed. Name search waits for a pause in typing, and text search
 * runs only when the user asks.
 */
class FilesViewModel(
    private val projectId: String,
    private val files: FilesGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(FilesUiState())
    private var generation = 0
    private var nameJob: Job? = null
    private var textJob: Job? = null

    /** The browser state. */
    val state: StateFlow<FilesUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            val name = callCore(io) { files.projectName(projectId) }.getOrNull()
            mutableState.update { it.copy(projectName = name) }
        }
        ensureLoaded(ROOT_DIR)
    }

    /** Loads [dir] unless its children are already cached or on their way. */
    fun ensureLoaded(dir: String) {
        when (mutableState.value.cache[dir]) {
            DirState.Loading, is DirState.Loaded -> return
            null, DirState.Failed -> load(dir)
        }
    }

    /** Asks the core for [dir] again after a failure. */
    fun retry(dir: String) = load(dir)

    /**
     * Turns hidden entries on or off. Every cached folder is dropped, and the root and the folders
     * in [expanded] load again with the new flag. Replies to the old flag are ignored.
     */
    fun setShowIgnored(
        show: Boolean,
        expanded: Set<String>,
    ) {
        if (show == mutableState.value.showIgnored) return
        generation++
        mutableState.update { it.copy(showIgnored = show, cache = emptyMap()) }
        load(ROOT_DIR)
        expanded.forEach { load(it) }
    }

    /** Opens or closes the search field. Closing clears the query and its results. */
    fun setSearchOpen(open: Boolean) {
        if (open) {
            mutableState.update { it.copy(searchOpen = true) }
            return
        }
        nameJob?.cancel()
        textJob?.cancel()
        mutableState.update {
            it.copy(
                searchOpen = false,
                query = "",
                nameHits = null,
                nameProblem = false,
                textSearch = TextSearch.Idle,
            )
        }
    }

    /** Sets the query and, after a pause in typing, searches the file names. */
    fun onQueryChange(text: String) {
        mutableState.update {
            it.copy(
                query = text,
                nameProblem = false,
                textSearch = TextSearch.Idle,
            )
        }
        nameJob?.cancel()
        val needle = text.trim()
        if (needle.isEmpty()) {
            mutableState.update { it.copy(nameHits = null) }
            return
        }
        nameJob =
            viewModelScope.launch {
                delay(NAME_DEBOUNCE_MS)
                val result = callCore(io) { files.searchFileNames(projectId, needle, NAME_LIMIT) }
                if (currentNeedle() == needle) {
                    mutableState.update {
                        it.copy(
                            nameHits = result.getOrDefault(emptyList()),
                            nameProblem = result.isFailure,
                        )
                    }
                }
            }
    }

    /** Searches the text of project files for the current query. A blank query does nothing. */
    fun searchText() {
        val needle = currentNeedle()
        if (needle.isEmpty()) return
        textJob?.cancel()
        mutableState.update { it.copy(textSearch = TextSearch.Loading) }
        textJob =
            viewModelScope.launch {
                val result = callCore(io) { files.searchFileText(projectId, needle, TEXT_LIMIT) }
                if (currentNeedle() == needle) {
                    val next =
                        result.fold(
                            onSuccess = { TextSearch.Hits(it) },
                            onFailure = { TextSearch.Failed },
                        )
                    mutableState.update { it.copy(textSearch = next) }
                }
            }
    }

    private fun currentNeedle(): String = mutableState.value.query.trim()

    private fun load(dir: String) {
        val epoch = generation
        val show = mutableState.value.showIgnored
        setCache(dir, DirState.Loading)
        viewModelScope.launch {
            val result = callCore(io) { files.listDir(projectId, dir, show) }
            if (epoch == generation) {
                setCache(
                    dir,
                    result.fold(
                        onSuccess = { DirState.Loaded(it.entries, it.truncated) },
                        onFailure = { DirState.Failed },
                    ),
                )
            }
        }
    }

    private fun setCache(
        dir: String,
        value: DirState,
    ) {
        mutableState.update { it.copy(cache = it.cache + (dir to value)) }
    }

    companion object {
        /** The pause in typing before a file name search runs. */
        const val NAME_DEBOUNCE_MS: Long = 300L

        /** The most file names one search returns. */
        const val NAME_LIMIT: Int = 50

        /** The most matching lines one text search returns. */
        const val TEXT_LIMIT: Int = 100

        /** Builds the browser model for the project with [projectId]. */
        fun factory(
            projectId: String,
            files: FilesGateway,
        ) = viewModelFactory {
            initializer { FilesViewModel(projectId, files) }
        }
    }
}
