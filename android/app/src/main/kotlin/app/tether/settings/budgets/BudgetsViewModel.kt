package app.tether.settings.budgets

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
 * What the budgets form shows. [daily] and [session] are the typed texts. [savedDaily] and
 * [savedSession] are the limits the core holds, so the form knows whether anything changed.
 */
@Immutable
data class BudgetsUiState(
    val loading: Boolean = true,
    val loadFailed: Boolean = false,
    val daily: String = "",
    val session: String = "",
    val savedDaily: Long? = null,
    val savedSession: Long? = null,
    val saving: Boolean = false,
    val saved: Boolean = false,
    val saveFailure: BudgetsProblem? = null,
) {
    /** How the daily text reads right now. */
    val dailyInput: DollarInput
        get() = readDollars(daily)

    /** How the session text reads right now. */
    val sessionInput: DollarInput
        get() = readDollars(session)

    /** The daily limit to save, or null for no limit. Null also when the text is invalid. */
    val dailyMicros: Long?
        get() = (dailyInput as? DollarInput.Valid)?.micros

    /** The session limit to save, or null for no limit. Null also when the text is invalid. */
    val sessionMicros: Long?
        get() = (sessionInput as? DollarInput.Valid)?.micros

    /** True when both fields read as a limit or as no limit. */
    val valid: Boolean
        get() = dailyInput !is DollarInput.Invalid && sessionInput !is DollarInput.Invalid

    /** True when either limit differs from the one the core holds. */
    val changed: Boolean
        get() = dailyMicros != savedDaily || sessionMicros != savedSession

    /** Save is allowed once the form loaded, is valid, changed, and is not already saving. */
    val canSave: Boolean
        get() = !loading && !loadFailed && !saving && valid && changed
}

/**
 * State for the budgets form. Reads and writes run on [io]. A save with an invalid or unchanged
 * form does nothing, so a double tap saves once.
 */
class BudgetsViewModel(
    private val gateway: BudgetsGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(BudgetsUiState())

    /** The screen state. */
    val state: StateFlow<BudgetsUiState> = mutableState.asStateFlow()

    init {
        load()
    }

    /** Reads the saved limits again after a failed load. */
    fun retry() {
        mutableState.update { it.copy(loading = true, loadFailed = false) }
        load()
    }

    /** The daily field changed to [text]. */
    fun onDailyChange(text: String) = edit { copy(daily = text) }

    /** The session field changed to [text]. */
    fun onSessionChange(text: String) = edit { copy(session = text) }

    /** Clears the daily limit. It is saved as no limit when the form is saved. */
    fun removeDaily() = edit { copy(daily = "") }

    /** Clears the session limit. It is saved as no limit when the form is saved. */
    fun removeSession() = edit { copy(session = "") }

    /** Saves both limits in one call. The form keeps its values when the save fails. */
    fun save() {
        val form = state.value
        if (!form.canSave) return
        val daily = form.dailyMicros
        val session = form.sessionMicros
        mutableState.update { it.copy(saving = true, saveFailure = null) }
        viewModelScope.launch {
            callCore(io) { gateway.setBudgets(daily, session) }
                .onSuccess {
                    mutableState.update {
                        it.copy(
                            saving = false,
                            saved = true,
                            savedDaily = daily,
                            savedSession = session,
                        )
                    }
                }.onFailure { failure ->
                    mutableState.update {
                        it.copy(saving = false, saveFailure = failure.budgetsProblem())
                    }
                }
        }
    }

    private fun edit(change: BudgetsUiState.() -> BudgetsUiState) {
        mutableState.update { it.change().copy(saved = false, saveFailure = null) }
    }

    private fun load() {
        viewModelScope.launch {
            callCore(io) { gateway.budgets() }
                .onSuccess { limits ->
                    mutableState.update {
                        it.copy(
                            loading = false,
                            loadFailed = false,
                            daily = editableOrBlank(limits.dailyMicros),
                            session = editableOrBlank(limits.sessionMicros),
                            savedDaily = limits.dailyMicros,
                            savedSession = limits.sessionMicros,
                        )
                    }
                }.onFailure {
                    mutableState.update { it.copy(loading = false, loadFailed = true) }
                }
        }
    }
}
