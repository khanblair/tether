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

/** The mode sheet's state. [confirming] shows the Bypass confirmation in place of the list. */
data class ModeSheetState(
    val current: SessionMode? = null,
    val confirming: Boolean = false,
    val bypassDenied: Boolean = false,
    val saving: Boolean = false,
    val failure: SessionFailure? = null,
)

/**
 * Sets the session's permission mode. Bypass needs a biometric check first; [authenticate] runs
 * it and calls back once, on the main thread. A denied check changes nothing.
 */
class ModeSheetViewModel(
    private val deps: SessionSheetDeps,
    private val authenticate: (onSuccess: () -> Unit, onError: () -> Unit) -> Unit,
    private val onApplied: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ModeSheetState())

    /** The screen state. */
    val state: StateFlow<ModeSheetState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            sessionCall(io) { deps.settings.details(deps.sessionId).mode }
                .onSuccess { mode -> mutableState.update { it.copy(current = mode) } }
                .onFailure { failure ->
                    mutableState.update { it.copy(failure = failure.sessionFailure()) }
                }
        }
    }

    /** The user picked [mode]. Bypass opens its confirmation; the other modes apply at once. */
    fun choose(mode: SessionMode) {
        if (deps.isRunning() || mutableState.value.saving) return
        if (mode == SessionMode.BYPASS) {
            mutableState.update { it.copy(confirming = true, bypassDenied = false) }
        } else {
            apply(mode)
        }
    }

    /** The user confirmed Bypass. Applies it only after the biometric check succeeds. */
    fun confirmBypass() {
        if (deps.isRunning() || mutableState.value.saving) return
        authenticate(
            { apply(SessionMode.BYPASS) },
            { mutableState.update { it.copy(bypassDenied = true) } },
        )
    }

    /** The user backed out of the Bypass confirmation. */
    fun cancelBypass() {
        mutableState.update { it.copy(confirming = false, bypassDenied = false) }
    }

    private fun apply(mode: SessionMode) {
        mutableState.update { it.copy(saving = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) {
                deps.settings.setMode(deps.sessionId, mode)
                deps.reopen()
            }.onSuccess {
                mutableState.update { it.copy(saving = false, confirming = false) }
                onApplied()
            }.onFailure { failure ->
                mutableState.update {
                    it.copy(saving = false, confirming = false, failure = failure.sessionFailure())
                }
            }
        }
    }

    companion object {
        /** The mode sheet's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            authenticate: (onSuccess: () -> Unit, onError: () -> Unit) -> Unit,
            onApplied: () -> Unit,
        ) = viewModelFactory { initializer { ModeSheetViewModel(deps, authenticate, onApplied) } }
    }
}
