package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.home.ProjectGateway
import app.tether.session.SessionMode
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** One project and its permission mode default. Null means the app default. */
@Immutable
data class ProjectModeRow(
    val id: String,
    val name: String,
    val mode: SessionMode?,
)

/** Which default the mode picker is choosing: the app default, or one project's. */
sealed interface ModeTarget {
    /** The app default for new sessions. */
    data object App : ModeTarget

    /** One project's default. */
    data class Project(
        val id: String,
        val name: String,
    ) : ModeTarget
}

/** What the permissions screen shows. */
@Immutable
data class PermissionsUiState(
    val defaultMode: SessionMode = SessionMode.ACCEPT_EDITS,
    val projects: List<ProjectModeRow> = emptyList(),
    val failed: Boolean = false,
    val picking: ModeTarget? = null,
)

/**
 * The default permission mode and the per-project mode. Bypass is never saved as a default: it
 * is turned on inside a session, with its biometric check, and ends with the session.
 */
class PermissionsViewModel(
    private val store: SettingsStore,
    private val projects: ProjectGateway,
    private val defaults: ProjectDefaultsGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(PermissionsUiState(defaultMode = store.defaultMode()))

    /** The screen state. */
    val state: StateFlow<PermissionsUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch(io) {
            runCatching {
                projects.list().map { info ->
                    ProjectModeRow(info.id, info.name, defaults.load(info.id).mode)
                }
            }.onSuccess { rows -> mutableState.update { it.copy(projects = rows) } }
                .onFailure { mutableState.update { it.copy(failed = true) } }
        }
    }

    /** Opens the picker for [target]. */
    fun open(target: ModeTarget) {
        mutableState.update { it.copy(picking = target) }
    }

    /** Closes the picker without a change. */
    fun close() {
        mutableState.update { it.copy(picking = null) }
    }

    /**
     * Saves [mode] for the picked target. Null means the app default, for a project. Bypass is
     * never saved, so a Bypass pick changes nothing.
     */
    fun choose(mode: SessionMode?) {
        val target = mutableState.value.picking ?: return
        mutableState.update { it.copy(picking = null) }
        if (mode != SessionMode.BYPASS) apply(target, mode)
    }

    private fun apply(
        target: ModeTarget,
        mode: SessionMode?,
    ) {
        viewModelScope.launch(io) {
            when (target) {
                ModeTarget.App -> applyAppMode(mode)
                is ModeTarget.Project -> applyProjectMode(target.id, mode)
            }
        }
    }

    private fun applyAppMode(mode: SessionMode?) {
        if (mode == null) return
        store.saveDefaultMode(mode)
        mutableState.update { it.copy(defaultMode = mode) }
    }

    private fun applyProjectMode(
        projectId: String,
        mode: SessionMode?,
    ) {
        runCatching {
            defaults.save(projectId, defaults.load(projectId).copy(mode = mode))
        }.onSuccess {
            mutableState.update { state ->
                state.copy(
                    projects =
                        state.projects.map { row ->
                            if (row.id == projectId) row.copy(mode = mode) else row
                        },
                )
            }
        }.onFailure { mutableState.update { it.copy(failed = true) } }
    }
}
