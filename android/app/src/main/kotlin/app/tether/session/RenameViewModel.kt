package app.tether.session

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The rename sheet's state. Saving is disabled while the title is blank. */
data class RenameState(
    val title: String = "",
    val saving: Boolean = false,
    val failure: SessionFailure? = null,
) {
    val canSave: Boolean
        get() = title.isNotBlank() && !saving
}

/** Renames the session to the trimmed title, then calls [onApplied] so the top bar reloads. */
class RenameViewModel(
    private val deps: SessionSheetDeps,
    private val onApplied: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(RenameState())

    /** The screen state. */
    val state: StateFlow<RenameState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            sessionCall(io) { deps.settings.details(deps.sessionId).title }
                .onSuccess { title -> mutableState.update { it.copy(title = title) } }
                .onFailure { failure ->
                    mutableState.update { it.copy(failure = failure.sessionFailure()) }
                }
        }
    }

    /** The user edited the title field. */
    fun onTitleChange(text: String) {
        mutableState.update { it.copy(title = text, failure = null) }
    }

    /** Saves the trimmed title. A blank title is never sent to the core. */
    fun save() {
        val trimmed = mutableState.value.title.trim()
        if (trimmed.isEmpty() || mutableState.value.saving) return
        mutableState.update { it.copy(saving = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) { deps.settings.rename(deps.sessionId, trimmed) }
                .onSuccess {
                    mutableState.update { it.copy(saving = false) }
                    onApplied()
                }.onFailure { failure ->
                    mutableState.update {
                        it.copy(saving = false, failure = failure.sessionFailure())
                    }
                }
        }
    }

    companion object {
        /** The rename sheet's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            onApplied: () -> Unit,
        ) = viewModelFactory { initializer { RenameViewModel(deps, onApplied) } }
    }
}
