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

/** Whether the scope puts workspace files back. Only the conversation scope leaves them alone. */
val RewindScope.restoresFiles: Boolean
    get() = this != RewindScope.CONVERSATION

/** Whether the scope puts the conversation back. Only the files scope leaves it alone. */
val RewindScope.restoresConversation: Boolean
    get() = this != RewindScope.FILES

/** A rewind that changes files needs a confirmation before it runs. */
val RewindScope.needsConfirmation: Boolean
    get() = restoresFiles

/** The rewind sheet's state. [confirming] asks before a files rewind runs. */
data class RewindState(
    val loading: Boolean = true,
    val checkpoints: List<CheckpointItem> = emptyList(),
    val selected: String? = null,
    val scope: RewindScope = RewindScope.FILES_AND_CONVERSATION,
    val confirming: Boolean = false,
    val applying: Boolean = false,
    val failure: SessionFailure? = null,
)

/**
 * Lists the checkpoints, newest first, and rewinds the chosen one in the chosen scope. A files
 * scope asks for a confirmation first. Success calls [onRewound] for the toast, then [onApplied].
 */
class RewindViewModel(
    private val deps: SessionSheetDeps,
    private val onApplied: () -> Unit,
    private val onRewound: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(RewindState())

    /** The screen state. */
    val state: StateFlow<RewindState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            sessionCall(io) {
                deps.settings
                    .checkpoints(deps.sessionId)
                    .sortedByDescending(CheckpointItem::createdAtMs)
            }.onSuccess { checkpoints ->
                mutableState.update {
                    it.copy(
                        loading = false,
                        checkpoints = checkpoints,
                        selected = checkpoints.firstOrNull()?.id,
                    )
                }
            }.onFailure { failure ->
                mutableState.update {
                    it.copy(loading = false, failure = failure.sessionFailure())
                }
            }
        }
    }

    /** The user picked a checkpoint. */
    fun select(checkpointId: String) {
        mutableState.update { it.copy(selected = checkpointId, failure = null) }
    }

    /** The user picked a restore scope. */
    fun setScope(scope: RewindScope) {
        mutableState.update { it.copy(scope = scope, failure = null) }
    }

    /** The user pressed the rewind button. Files scopes ask first; the conversation scope runs. */
    fun requestRewind() {
        val current = mutableState.value
        if (current.applying || current.selected == null) return
        if (current.scope.needsConfirmation) {
            mutableState.update { it.copy(confirming = true) }
        } else {
            apply()
        }
    }

    /** The user confirmed a files rewind. */
    fun confirmRewind() {
        mutableState.update { it.copy(confirming = false) }
        apply()
    }

    /** The user backed out of the confirmation. */
    fun dismissConfirm() {
        mutableState.update { it.copy(confirming = false) }
    }

    private fun apply() {
        val current = mutableState.value
        val checkpointId = current.selected ?: return
        val scope = current.scope
        mutableState.update { it.copy(applying = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) { deps.rewinder.rewind(checkpointId, scope) }
                .onSuccess {
                    mutableState.update { it.copy(applying = false) }
                    onRewound()
                    onApplied()
                }.onFailure { failure ->
                    mutableState.update {
                        it.copy(applying = false, failure = failure.sessionFailure())
                    }
                }
        }
    }

    companion object {
        /** The rewind sheet's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            onApplied: () -> Unit,
            onRewound: () -> Unit,
        ) = viewModelFactory { initializer { RewindViewModel(deps, onApplied, onRewound) } }
    }
}
