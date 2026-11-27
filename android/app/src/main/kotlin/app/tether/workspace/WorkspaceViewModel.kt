package app.tether.workspace

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ProjectRuntimeException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The workspace screen state. */
@Immutable
data class WorkspaceUiState(
    val location: WorkspaceLocation? = null,
    val dataPaths: WorkspaceDataPaths = WorkspaceDataPaths(),
    val choice: WorkspaceChoice = WorkspaceChoice.Private,
    val pathInput: String = "",
    val problem: WorkspaceProblem? = null,
    val loading: Boolean = true,
    val saving: Boolean = false,
    val allFilesAccessGranted: Boolean = false,
    val permissionRequired: Boolean = false,
)

/** Holds the workspace choice and keeps core calls off the main thread. */
class WorkspaceViewModel(
    private val gateway: WorkspaceGateway,
    private val sharedFolderAllowed: Boolean,
    initialAllFilesAccessGranted: Boolean,
    private val suggestedPath: String,
    private val paths: (() -> WorkspaceDataPaths)? = null,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow(
            WorkspaceUiState(
                allFilesAccessGranted = initialAllFilesAccessGranted,
            ),
        )

    /** The current screen state. */
    val state: StateFlow<WorkspaceUiState> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads the current location and the known data paths. */
    fun refresh() {
        viewModelScope.launch(io) {
            val location =
                try {
                    gateway.location()
                } catch (_: ProjectRuntimeException) {
                    mutableState.update { state -> state.copy(loading = false) }
                    return@launch
                }
            val dataPaths =
                try {
                    paths?.invoke() ?: WorkspaceDataPaths()
                } catch (_: ProjectRuntimeException) {
                    WorkspaceDataPaths()
                }
            mutableState.update {
                it.copy(
                    location = location,
                    dataPaths = dataPaths,
                    choice =
                        if (location.newProjectRoot == null || !sharedFolderAllowed) {
                            WorkspaceChoice.Private
                        } else {
                            WorkspaceChoice.Custom
                        },
                    pathInput = location.newProjectRoot ?: suggestedPath,
                    loading = false,
                )
            }
        }
    }

    /** Changes the private or custom choice. */
    fun choose(choice: WorkspaceChoice) {
        if (choice == WorkspaceChoice.Custom && !sharedFolderAllowed) return
        mutableState.update {
            it.copy(
                choice = choice,
                problem = null,
                permissionRequired = false,
            )
        }
    }

    /** Changes the custom path and reports the two cheap local checks immediately. */
    fun onPathChange(value: String) {
        mutableState.update {
            it.copy(
                pathInput = value,
                problem = validateWorkspacePath(value),
                permissionRequired = false,
            )
        }
    }

    /** Updates the permission state after returning from Android settings. */
    fun onAllFilesAccessChanged(granted: Boolean) {
        mutableState.update { it.copy(allFilesAccessGranted = granted, permissionRequired = false) }
    }

    /** Saves the selected choice and invokes [onSaved] only after the core accepts it. */
    fun save(onSaved: () -> Unit = {}) {
        val current = mutableState.value
        if (current.choice == WorkspaceChoice.Custom && !current.allFilesAccessGranted) {
            mutableState.update { it.copy(permissionRequired = true) }
            return
        }
        val path =
            if (current.choice == WorkspaceChoice.Private) {
                null
            } else {
                current.pathInput.trim()
            }
        val localProblem = path?.let(::validateWorkspacePath)
        if (localProblem != null) {
            mutableState.update { it.copy(problem = localProblem) }
            return
        }
        mutableState.update { it.copy(problem = null, permissionRequired = false, saving = true) }
        viewModelScope.launch(io) {
            val problem =
                try {
                    gateway.setNewProjectRoot(path)
                } catch (_: ProjectRuntimeException) {
                    WorkspaceProblem.Unavailable
                }
            mutableState.update { it.copy(problem = problem, saving = false) }
            if (problem == null) {
                refresh()
                viewModelScope.launch { onSaved() }
            }
        }
    }

    /** Saves the private default. Existing projects are never changed by this action. */
    fun usePrivateStorage(onSaved: () -> Unit = {}) {
        choose(WorkspaceChoice.Private)
        save(onSaved)
    }

    companion object {
        /** Builds the screen state holder for a navigation destination. */
        fun factory(
            gateway: WorkspaceGateway,
            sharedFolderAllowed: Boolean,
            initialAllFilesAccessGranted: Boolean,
            suggestedPath: String,
            paths: () -> WorkspaceDataPaths,
        ) = viewModelFactory {
            initializer {
                WorkspaceViewModel(
                    gateway = gateway,
                    sharedFolderAllowed = sharedFolderAllowed,
                    initialAllFilesAccessGranted = initialAllFilesAccessGranted,
                    suggestedPath = suggestedPath,
                    paths = paths,
                )
            }
        }
    }
}

/** The default shared folder, computed from Android's external storage root. */
fun suggestedWorkspaceRoot(externalStorageRoot: String): String =
    java.io.File(externalStorageRoot, SHARED_FOLDER_NAME).absolutePath

/** Cheap path checks that do not need the core or the filesystem. */
fun validateWorkspacePath(path: String): WorkspaceProblem? =
    when {
        path.isBlank() -> WorkspaceProblem.EmptyPath
        !path.startsWith("/") -> WorkspaceProblem.RootNotAbsolute
        else -> null
    }

private const val SHARED_FOLDER_NAME = "Tether"
