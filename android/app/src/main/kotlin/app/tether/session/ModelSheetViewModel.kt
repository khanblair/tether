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

/** Whether this model is the current one and reports reasoning support. */
private fun ModelRef?.reasons(groups: List<ModelGroup>): Boolean {
    val ref = this ?: return false
    return groups.any { group ->
        group.provider == ref.provider &&
            group.models.any { it.id == ref.modelId && it.reasoning == true }
    }
}

/** A model can run agent sessions unless its catalog entry says it has no tool support. */
val ModelChoice.canRunAgents: Boolean
    get() = tools != false

/** The provider and model the session uses now. */
data class ModelRef(
    val provider: ProviderView,
    val modelId: String,
)

/**
 * The model sheet's state. [failure] is null when the last call worked. [effort] is the session's
 * thinking effort, null for the model default. [showsEffort] is true only when the current model
 * reports reasoning support. [query] is the search text that filters the model list.
 */
data class ModelSheetState(
    val loading: Boolean = true,
    val groups: List<ModelGroup> = emptyList(),
    val current: ModelRef? = null,
    val saving: Boolean = false,
    val failure: SessionFailure? = null,
    val effort: ThinkingEffort? = null,
    val showsEffort: Boolean = false,
    val query: String = "",
)

private val WORD_BREAK = Regex("\\s+")

/**
 * The groups that match [query]. Every word of the query must appear in a model's display name or
 * id, ignoring case. An empty query keeps every group, and a provider with no match is dropped.
 */
internal fun filterModelGroups(
    groups: List<ModelGroup>,
    query: String,
): List<ModelGroup> {
    val words =
        query
            .trim()
            .lowercase()
            .split(WORD_BREAK)
            .filter { it.isNotEmpty() }
    if (words.isEmpty()) return groups
    return groups.mapNotNull { group ->
        val models = group.models.filter { model -> model.matches(words) }
        if (models.isEmpty()) null else group.copy(models = models)
    }
}

private fun ModelChoice.matches(words: List<String>): Boolean {
    val text = "$displayName $id".lowercase()
    return words.all { it in text }
}

/**
 * The thinking section shows while the query is empty, or while the current model is still in
 * [visible]. A model that cannot reason never shows it.
 */
internal fun showsEffortSection(
    state: ModelSheetState,
    visible: List<ModelGroup>,
): Boolean {
    val current = state.current
    val currentVisible =
        current != null &&
            visible.any { group ->
                group.provider == current.provider &&
                    group.models.any { it.id == current.modelId }
            }
    return state.showsEffort && (state.query.isBlank() || currentVisible)
}

/** What the sheet reads when it opens: the session's model and effort, and the saved models. */
private data class SheetCatalog(
    val current: ModelRef,
    val effort: ThinkingEffort?,
    val groups: List<ModelGroup>,
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
                .onSuccess { catalog ->
                    mutableState.update {
                        it.copy(
                            loading = false,
                            current = catalog.current,
                            groups = catalog.groups,
                            effort = catalog.effort,
                            showsEffort = catalog.current.reasons(catalog.groups),
                        )
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
        save {
            deps.settings.setModel(deps.sessionId, provider, model.id)
            deps.reopen()
        }
    }

    /** Sets the search text that filters the model list. */
    fun onQueryChange(query: String) {
        mutableState.update { it.copy(query = query) }
    }

    /**
     * Sets the thinking effort for the session, or the model default for null. Ignored while a
     * turn or a save is running, and for a model that does not report reasoning support.
     */
    fun chooseEffort(effort: ThinkingEffort?) {
        val state = mutableState.value
        if (deps.isRunning() || state.saving || !state.showsEffort) return
        save {
            deps.settings.setEffort(deps.sessionId, effort)
            deps.reopen()
        }
    }

    /** Runs [change] off the main thread. [onApplied] runs on success, else a failure shows. */
    private fun save(change: () -> Unit) {
        mutableState.update { it.copy(saving = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) { change() }
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

    private fun catalog(): SheetCatalog {
        val details = deps.settings.details(deps.sessionId)
        val groups =
            ProviderView.entries
                .filter { deps.providers.hasKey(it) }
                .map { ModelGroup(it, deps.settings.models(it)) }
                .filter { it.models.isNotEmpty() }
        return SheetCatalog(ModelRef(details.provider, details.modelId), details.effort, groups)
    }

    companion object {
        /** The model sheet's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            onApplied: () -> Unit,
        ) = viewModelFactory { initializer { ModelSheetViewModel(deps, onApplied) } }
    }
}
