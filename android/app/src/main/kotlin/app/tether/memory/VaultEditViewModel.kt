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

/** How the typed text is saved: it replaces the whole note, or it is added to the end. */
enum class VaultEditMode {
    /** The text is the whole note, frontmatter included. */
    Replace,

    /** The text is added to the end of the existing note. */
    Append,
}

/**
 * What the note editor shows. [existing] is true when the note is already in the vault; only then
 * can the user choose [VaultEditMode.Append]. [savedPath] is set once the note is written.
 */
@Immutable
data class VaultEditUiState(
    val path: String = "",
    val source: String = "",
    val mode: VaultEditMode = VaultEditMode.Replace,
    val existing: Boolean = false,
    val loading: Boolean = true,
    val saving: Boolean = false,
    val pathProblem: AreaProblem? = null,
    val problem: AreaProblem? = null,
    val savedPath: String? = null,
)

/**
 * State holder for the note editor. [notePath] is null for a new note. The path is checked here
 * before the core sees it, so the reason shows under the field.
 */
class VaultEditViewModel(
    private val gateway: VaultGateway,
    private val notePath: String?,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(VaultEditUiState(path = notePath.orEmpty()))

    /** The note text as the vault has it, kept while the editor is in Append mode. */
    private var savedSource = ""

    /** The editor state. */
    val state: StateFlow<VaultEditUiState> = mutableState.asStateFlow()

    init {
        if (notePath == null) {
            mutableState.value = mutableState.value.copy(loading = false)
        } else {
            viewModelScope.launch {
                val result = callCore(io) { gateway.read(notePath) }
                mutableState.value =
                    result.fold(
                        onSuccess = { note ->
                            savedSource = note.source
                            mutableState.value.copy(
                                source = note.source,
                                existing = true,
                                loading = false,
                            )
                        },
                        onFailure = {
                            mutableState.value.copy(
                                loading = false,
                                problem = it.areaProblem(),
                            )
                        },
                    )
            }
        }
    }

    /** Records the note path. A new note can still change its path; an existing one is fixed. */
    fun onPathChange(text: String) {
        mutableState.value =
            mutableState.value.copy(path = text, pathProblem = null, problem = null)
    }

    /** Records the note text. */
    fun onSourceChange(text: String) {
        mutableState.value = mutableState.value.copy(source = text, problem = null)
    }

    /**
     * Chooses whether the text replaces the note or is added to its end. Append starts with an
     * empty field; switching back shows the note text again.
     */
    fun onModeChange(mode: VaultEditMode) {
        val current = mutableState.value
        if (mode == current.mode) return
        val source = if (mode == VaultEditMode.Append) "" else savedSource
        if (current.mode == VaultEditMode.Replace) savedSource = current.source
        mutableState.value = current.copy(mode = mode, source = source, problem = null)
    }

    /** Saves the note. Nothing is sent when the path or the text is not usable. */
    fun save() {
        val current = mutableState.value
        val path = current.path.trim()
        if (current.saving || current.source.isBlank()) return
        val pathProblem = vaultPathProblem(path)
        if (pathProblem != null) {
            mutableState.value = current.copy(pathProblem = pathProblem)
            return
        }
        mutableState.value = current.copy(saving = true, pathProblem = null, problem = null)
        viewModelScope.launch {
            val result =
                callCore(io) {
                    if (current.mode == VaultEditMode.Append && current.existing) {
                        gateway.append(path, current.source)
                    } else {
                        gateway.write(path, current.source)
                    }
                }
            mutableState.value =
                result.fold(
                    onSuccess = { mutableState.value.copy(saving = false, savedPath = path) },
                    onFailure = {
                        mutableState.value.copy(
                            saving = false,
                            problem = it.areaProblem(),
                        )
                    },
                )
        }
    }
}
