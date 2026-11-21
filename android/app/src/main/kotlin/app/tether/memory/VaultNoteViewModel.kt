package app.tether.memory

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.memory.BodyPart.BodyLink
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * What the note screen shows. [parts] is the body with its links marked resolved or missing.
 * [backlinks] are the links from other notes into this one. [linkFailed] is set when a resolved
 * link's note could not be opened.
 */
@Immutable
data class VaultNoteUiState(
    val loading: Boolean = true,
    val note: VaultNote? = null,
    val parts: List<BodyPart> = emptyList(),
    val backlinks: List<VaultLink> = emptyList(),
    val problem: AreaProblem? = null,
    val linkFailed: Boolean = false,
)

/** State holder for one note: its body, its links, and the notes that link to it. */
class VaultNoteViewModel(
    private val gateway: VaultGateway,
    private val path: String,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(VaultNoteUiState())

    /** The screen state. */
    val state: StateFlow<VaultNoteUiState> = mutableState.asStateFlow()

    init {
        load()
    }

    /** Reads the note and its links again. */
    fun load() {
        viewModelScope.launch {
            val result = callCore(io) { gateway.read(path) to gateway.links(path) }
            mutableState.value =
                result.fold(
                    onSuccess = { (note, links) ->
                        VaultNoteUiState(
                            loading = false,
                            note = note,
                            parts =
                                resolveBodyLinks(
                                    parseBody(noteBody(note.source)),
                                    links.outgoing,
                                ),
                            backlinks = links.incoming,
                        )
                    },
                    onFailure = { VaultNoteUiState(loading = false, problem = it.areaProblem()) },
                )
        }
    }

    /**
     * Opens the note a resolved [link] points to. The core gives no path for a link, so the
     * candidate paths are tried in order. [onOpen] gets the path that was found.
     */
    fun openLink(
        link: BodyLink,
        onOpen: (String) -> Unit,
    ) {
        if (!link.resolved) return
        mutableState.value = mutableState.value.copy(linkFailed = false)
        viewModelScope.launch {
            val found = callCore(io) { firstReadable(linkCandidates(path, link.target)) }
            found.fold(
                onSuccess = { target -> if (target != null) onOpen(target) else markLinkFailed() },
                onFailure = { markLinkFailed() },
            )
        }
    }

    private fun markLinkFailed() {
        mutableState.value = mutableState.value.copy(linkFailed = true)
    }

    private fun firstReadable(candidates: List<String>): String? =
        candidates.firstOrNull { candidate ->
            try {
                gateway.read(candidate)
                true
            } catch (missing: AreaFailure) {
                if (missing.problem == AreaProblem.NotFound) false else throw missing
            }
        }
}
