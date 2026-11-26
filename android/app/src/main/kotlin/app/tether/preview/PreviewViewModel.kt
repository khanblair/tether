package app.tether.preview

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.files.FilesFailure
import app.tether.files.FilesProblem
import app.tether.files.filesProblem
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The largest file the preview serves, in bytes (10 MiB). */
private const val PREVIEW_LIMIT_BYTES = 10L * 1024L * 1024L

/** The bytes read to check that the file exists. Its content is not used. */
private const val PROBE_BYTES = 1L

/** What the WebView loads: a page of the project, or HTML held in memory. */
sealed interface PreviewLoad {
    /** The first read has not finished. */
    data object Loading : PreviewLoad

    /** A page of the project, at [url] on the preview origin. */
    data class Page(
        val url: String,
    ) : PreviewLoad

    /** HTML held in memory. */
    data class Html(
        val html: String,
    ) : PreviewLoad

    /** The file does not exist. */
    data object Missing : PreviewLoad

    /** The inline preview is no longer held, so it was dropped from memory. */
    data object Expired : PreviewLoad

    /** The file could not be read. [problem] says why. */
    data class Failed(
        val problem: FilesProblem,
    ) : PreviewLoad
}

/**
 * What the preview screen shows. [blocked] holds each distinct blocked URL. [reloadKey] changes
 * when the WebView must be built again, so the page loads with the current settings. An inline
 * preview has [networkLocked] set: its network stays off and cannot be turned on.
 */
@Immutable
data class PreviewUiState(
    val load: PreviewLoad = PreviewLoad.Loading,
    val networkAllowed: Boolean = false,
    val networkLocked: Boolean = false,
    val blocked: Set<String> = emptySet(),
    val reloadKey: Int = 0,
)

/**
 * State holder for one preview. [seedNetwork] reads the project's network setting once, and a
 * failed read counts as off. The WebView reads [networkAllowed] and [readFile] on its own thread.
 */
class PreviewViewModel(
    val target: PreviewTarget,
    private val files: PreviewFilesGateway,
    private val seedNetwork: () -> Boolean,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(PreviewUiState(networkLocked = target is PreviewTarget.Inline))
    private var userChangedNetwork = false

    /** The preview state. */
    val state: StateFlow<PreviewUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            if (target is PreviewTarget.File) {
                val seeded = callCore(io) { seedNetwork() }.getOrDefault(false)
                mutableState.update {
                    if (userChangedNetwork) it else it.copy(networkAllowed = seeded)
                }
            }
            val load = prepare()
            mutableState.update { it.copy(load = load) }
        }
    }

    /** Whether the network is on for this preview. Safe to read from any thread. */
    fun networkAllowed(): Boolean = mutableState.value.networkAllowed

    /** Turns the network on or off for this preview only, and loads the page again. */
    fun setNetworkAllowed(allowed: Boolean) {
        if (mutableState.value.networkLocked) return
        userChangedNetwork = true
        mutableState.update {
            it.copy(networkAllowed = allowed, blocked = emptySet(), reloadKey = it.reloadKey + 1)
        }
    }

    /** Checks the file again and loads the page. The blocked count starts over. */
    fun reload() {
        mutableState.update {
            it.copy(
                load = PreviewLoad.Loading,
                blocked = emptySet(),
                reloadKey = it.reloadKey + 1,
            )
        }
        viewModelScope.launch {
            val load = prepare()
            mutableState.update { it.copy(load = load) }
        }
    }

    /** Counts one blocked request. Each distinct URL counts once. Call on the main thread. */
    fun onBlocked(url: String) {
        mutableState.update { it.copy(blocked = it.blocked + url) }
    }

    /**
     * The bytes of [path] in the project, up to the preview limit. Blocking, so the WebView calls
     * it off the main thread. A preview with no project has no files.
     */
    fun readFile(path: String): FileBytes =
        when (val current = target) {
            is PreviewTarget.File ->
                files.readBytes(current.projectId, path, PREVIEW_LIMIT_BYTES)
            is PreviewTarget.Inline -> throw FilesFailure(FilesProblem.NotFound)
        }

    private suspend fun prepare(): PreviewLoad =
        when (val current = target) {
            is PreviewTarget.Inline -> PreviewLoad.Html(current.html)
            is PreviewTarget.File -> probe(current)
        }

    private suspend fun probe(current: PreviewTarget.File): PreviewLoad =
        callCore(io) { files.readBytes(current.projectId, current.path, PROBE_BYTES) }
            .fold(
                onSuccess = { PreviewLoad.Page(previewUrl(current.path)) },
                onFailure = { failure ->
                    when (val problem = failure.filesProblem()) {
                        FilesProblem.NotFound -> PreviewLoad.Missing
                        else -> PreviewLoad.Failed(problem)
                    }
                },
            )

    companion object {
        /** Builds the preview model for [target]. [seedNetwork] reads the project's setting. */
        fun factory(
            target: PreviewTarget,
            files: PreviewFilesGateway,
            seedNetwork: () -> Boolean,
        ) = viewModelFactory {
            initializer { PreviewViewModel(target, files, seedNetwork) }
        }
    }
}
