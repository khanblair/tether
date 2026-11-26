package app.tether.session

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ProviderView
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The models of one provider that has a saved key. */
data class ModelGroup(
    val provider: ProviderView,
    val models: List<ModelChoice>,
)

/** A model can run agent sessions unless its catalog entry says it has no tool support. */
val ModelChoice.canRunAgents: Boolean
    get() = tools != false

/** The provider and model the session uses now. */
data class ModelRef(
    val provider: ProviderView,
    val modelId: String,
)

/** The model sheet's state. [failure] is null when the last call worked. */
data class ModelSheetState(
    val loading: Boolean = true,
    val groups: List<ModelGroup> = emptyList(),
    val current: ModelRef? = null,
    val saving: Boolean = false,
    val failure: SessionFailure? = null,
)

/**
 * Lists the models of the providers that have a saved key, and sets the session's model. Choosing
 * a model reopens the runner, then calls [onApplied] so the screen reloads and closes the sheet.
 */
class ModelSheetViewModel(
    private val deps: SessionSheetDeps,
    private val onApplied: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ModelSheetState())

    /** The screen state. */
    val state: StateFlow<ModelSheetState> = mutableState.asStateFlow()

    init {
        load()
    }

    /** Reads the session's model and the saved models of each provider with a key. */
    fun load() {
        mutableState.update { it.copy(loading = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) { catalog() }
                .onSuccess { (current, groups) ->
                    mutableState.update {
                        it.copy(loading = false, current = current, groups = groups)
                    }
                }.onFailure { failure ->
                    mutableState.update {
                        it.copy(loading = false, failure = failure.sessionFailure())
                    }
                }
        }
    }

    /**
     * Sets [model] of [provider] for the session. Ignored while a turn or a save is running, and
     * for a model that cannot run agent sessions.
     */
    fun choose(
        provider: ProviderView,
        model: ModelChoice,
    ) {
        if (deps.isRunning() || mutableState.value.saving || !model.canRunAgents) return
        mutableState.update { it.copy(saving = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) {
                deps.settings.setModel(deps.sessionId, provider, model.id)
                deps.reopen()
            }.onSuccess {
                mutableState.update { it.copy(saving = false) }
                onApplied()
            }.onFailure { failure ->
                mutableState.update {
                    it.copy(saving = false, failure = failure.sessionFailure())
                }
            }
        }
    }

    private fun catalog(): Pair<ModelRef, List<ModelGroup>> {
        val details = deps.settings.details(deps.sessionId)
        val groups =
            ProviderView.entries
                .filter { deps.providers.hasKey(it) }
                .map { ModelGroup(it, deps.settings.models(it)) }
                .filter { it.models.isNotEmpty() }
        return ModelRef(details.provider, details.modelId) to groups
    }

    companion object {
        /** The model sheet's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            onApplied: () -> Unit,
        ) = viewModelFactory { initializer { ModelSheetViewModel(deps, onApplied) } }
    }
}
