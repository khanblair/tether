package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.core.generated.ProviderView
import app.tether.home.ProjectGateway
import app.tether.providers.ProviderGateway
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The models saved for one provider that has a key. */
@Immutable
data class ProviderModels(
    val provider: ProviderView,
    val models: List<CatalogModel>,
)

/** One project and the model it starts new sessions with. Null means the app default. */
@Immutable
data class ProjectModelRow(
    val id: String,
    val name: String,
    val model: DefaultModel?,
)

/** Which default the model picker is choosing: the app default, or one project's. */
sealed interface ModelTarget {
    /** The app default for new sessions. */
    data object App : ModelTarget

    /** One project's default. */
    data class Project(
        val id: String,
        val name: String,
    ) : ModelTarget
}

/**
 * What the models screen shows. [failed] is true when a saved list or project could not be read.
 */
@Immutable
data class ModelDefaultsUiState(
    val loading: Boolean = true,
    val appDefault: DefaultModel? = null,
    val providers: List<ProviderModels> = emptyList(),
    val projects: List<ProjectModelRow> = emptyList(),
    val failed: Boolean = false,
    val picking: ModelTarget? = null,
)

/**
 * The app default model and the per-project model. Only models of providers with a saved key are
 * offered. Reads and writes block, so they run on [io].
 */
class ModelDefaultsViewModel(
    private val store: SettingsStore,
    private val providers: ProviderGateway,
    private val catalog: ModelCatalogGateway,
    private val projects: ProjectGateway,
    private val defaults: ProjectDefaultsGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(ModelDefaultsUiState(appDefault = store.defaultModel()))

    /** The screen state. */
    val state: StateFlow<ModelDefaultsUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch(io) { load() }
    }

    /** Opens the picker for [target]. */
    fun open(target: ModelTarget) {
        mutableState.update { it.copy(picking = target) }
    }

    /** Closes the picker without a change. */
    fun close() {
        mutableState.update { it.copy(picking = null) }
    }

    /** Saves [model] for the picked target. Null means the app default, for a project. */
    fun choose(model: DefaultModel?) {
        val target = mutableState.value.picking ?: return
        mutableState.update { it.copy(picking = null) }
        viewModelScope.launch(io) {
            when (target) {
                ModelTarget.App -> applyAppDefault(model)
                is ModelTarget.Project -> applyProjectDefault(target.id, model)
            }
        }
    }

    private fun load() {
        val keyed = ProviderView.entries.filter(providers::hasKey)
        val lists =
            runCatching {
                keyed
                    .map {
                        ProviderModels(
                            it,
                            catalog.models(it),
                        )
                    }.filter { it.models.isNotEmpty() }
            }
        val rows = runCatching { projects.list().map { projectRow(it.id, it.name) } }
        mutableState.update {
            it.copy(
                loading = false,
                providers = lists.getOrDefault(emptyList()),
                projects = rows.getOrDefault(emptyList()),
                failed = lists.isFailure || rows.isFailure,
            )
        }
    }

    private fun projectRow(
        id: String,
        name: String,
    ): ProjectModelRow {
        val saved = defaults.load(id)
        val model =
            if (saved.provider != null && saved.modelId != null) {
                DefaultModel(saved.provider, saved.modelId)
            } else {
                null
            }
        return ProjectModelRow(id, name, model)
    }

    private fun applyAppDefault(model: DefaultModel?) {
        store.saveDefaultModel(model)
        mutableState.update { it.copy(appDefault = model) }
    }

    private fun applyProjectDefault(
        projectId: String,
        model: DefaultModel?,
    ) {
        runCatching {
            val current = defaults.load(projectId)
            defaults.save(
                projectId,
                current.copy(provider = model?.provider, modelId = model?.modelId),
            )
        }.onSuccess {
            mutableState.update { state ->
                state.copy(
                    projects =
                        state.projects.map { row ->
                            if (row.id == projectId) row.copy(model = model) else row
                        },
                )
            }
        }.onFailure { mutableState.update { it.copy(failed = true) } }
    }
}
