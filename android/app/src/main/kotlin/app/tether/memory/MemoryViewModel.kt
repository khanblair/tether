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
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * What the Memory screen shows. [entries] are the file's entries in order and [used] counts the
 * characters the file holds against [cap]. [editing] is the index of the entry being edited, if
 * any. [problem] is the reason the last change was refused, shown inline.
 */
@Immutable
data class MemoryUiState(
    val file: MemoryFile = MemoryFile.Memory,
    val entries: List<String> = emptyList(),
    val used: Int = 0,
    val loading: Boolean = true,
    val readFailed: Boolean = false,
    val draft: String = "",
    val editing: Int? = null,
    val editText: String = "",
    val problem: AreaProblem? = null,
) {
    /** The character limit of [file]. */
    val cap: Int get() = file.cap
}

/**
 * State holder for the Memory screen. Every read and write runs on [io], because the core blocks.
 * A refused change keeps what the user typed and sets [MemoryUiState.problem].
 */
class MemoryViewModel(
    private val gateway: MemoryGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(MemoryUiState())

    /** The screen state. */
    val state: StateFlow<MemoryUiState> = mutableState.asStateFlow()

    init {
        load(MemoryFile.Memory)
    }

    /** Shows [file] and reads it again. Changing the file cancels any edit in progress. */
    fun select(file: MemoryFile) {
        if (file == mutableState.value.file) return
        mutableState.value = MemoryUiState(file = file)
        load(file)
    }

    /** Records the text of the new-entry field. */
    fun onDraftChange(text: String) {
        mutableState.update { it.copy(draft = text, problem = null) }
    }

    /** Adds the draft as a new entry. A blank draft does nothing. */
    fun add() {
        val entry = mutableState.value.draft.trim()
        if (entry.isEmpty()) return
        write(MemoryEdit.Add(entry)) { it.copy(draft = "") }
    }

    /** Opens the entry at [index] for editing, with its text in the edit field. */
    fun startEdit(index: Int) {
        val text = mutableState.value.entries.getOrNull(index) ?: return
        mutableState.update { it.copy(editing = index, editText = text, problem = null) }
    }

    /** Records the text of the edit field. */
    fun onEditTextChange(text: String) {
        mutableState.update { it.copy(editText = text, problem = null) }
    }

    /** Closes the edit field and drops its text. */
    fun cancelEdit() {
        mutableState.update { it.copy(editing = null, editText = "", problem = null) }
    }

    /** Saves the edited entry. A blank text is refused; an unchanged one only closes the field. */
    fun saveEdit() {
        val current = mutableState.value
        val index = current.editing ?: return
        val old = current.entries.getOrNull(index) ?: return
        val new = current.editText.trim()
        when {
            new.isEmpty() -> mutableState.update { it.copy(problem = AreaProblem.InvalidEntry) }
            new == old -> cancelEdit()
            else -> write(MemoryEdit.Replace(old, new)) { it.copy(editing = null, editText = "") }
        }
    }

    /** Removes the entry at [index]. */
    fun remove(index: Int) {
        val entry = mutableState.value.entries.getOrNull(index) ?: return
        write(MemoryEdit.Remove(entry)) { it }
    }

    private fun load(file: MemoryFile) {
        viewModelScope.launch {
            val result = callCore(io) { gateway.read(file) }
            mutableState.update { state ->
                if (state.file != file) {
                    state
                } else {
                    result.fold(
                        onSuccess = { state.withContent(it) },
                        onFailure = {
                            state.copy(
                                loading = false,
                                readFailed = true,
                                problem = it.areaProblem(),
                            )
                        },
                    )
                }
            }
        }
    }

    private fun write(
        edit: MemoryEdit,
        onSaved: (MemoryUiState) -> MemoryUiState,
    ) {
        val file = mutableState.value.file
        viewModelScope.launch {
            val result = callCore(io) { gateway.apply(file, edit) }
            mutableState.update { state ->
                if (state.file != file) {
                    state
                } else {
                    result.fold(
                        onSuccess = { onSaved(state.withContent(it)) },
                        onFailure = { state.copy(problem = it.areaProblem()) },
                    )
                }
            }
        }
    }

    private fun MemoryUiState.withContent(content: String): MemoryUiState =
        copy(
            entries = splitEntries(content),
            used = characterCount(content),
            loading = false,
            readFailed = false,
            problem = null,
        )
}
