package app.tether.newsession

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.home.ProjectSummary
import app.tether.home.ProjectsSource
import app.tether.session.SessionMode
import app.tether.session.SessionStartGateway
import app.tether.session.StartResult
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.launch

/** A visible start failure that does not create a session. */
enum class NewSessionStartError {
    NoKey,
    NoModels,
    Failed,
}

/** What the New session screen can show. */
sealed interface NewSessionUiState {
    /** Projects and the selected project's preview are loading. */
    data object Loading : NewSessionUiState

    /** There is no saved project to start a session in. */
    data object NoProject : NewSessionUiState

    /** A project is selected and its context is ready to review. */
    @Immutable
    data class Ready(
        val project: ProjectSummary,
        val preview: NewSessionPreview,
        val modelId: String?,
        val mode: SessionMode,
        val starting: Boolean = false,
        val startError: NewSessionStartError? = null,
        val projects: List<ProjectSummary> = emptyList(),
    ) : NewSessionUiState

    /** The project list or preview could not be read. */
    data object Failed : NewSessionUiState
}

/** Loads the selected project's context and creates a session only after the first Send. */
class NewSessionViewModel(
    private val projects: ProjectsSource,
    private val previews: NewSessionGateway,
    private val starter: SessionStartGateway,
    private val pending: PendingFirstMessage,
    private val requestedProjectId: String? = null,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow<NewSessionUiState>(NewSessionUiState.Loading)
    private val opened = Channel<String>(Channel.BUFFERED)
    private var allProjects: List<ProjectSummary> = emptyList()
    private var loadVersion = 0

    /** The screen state. */
    val state: StateFlow<NewSessionUiState> = mutableState.asStateFlow()

    /** Each started session id is emitted once for navigation. */
    val openedSessions: Flow<String> = opened.receiveAsFlow()

    init {
        loadProjects()
    }

    /** Chooses a project and reloads only its preview. */
    fun selectProject(projectId: String) {
        val project = allProjects.firstOrNull { it.id == projectId } ?: return
        val ready = mutableState.value as? NewSessionUiState.Ready
        val busy = ready?.starting == true
        if (busy || ready?.project?.id == project.id) return
        loadPreview(project)
    }

    /** Reads the project list and selected preview again after a visible load failure. */
    fun retry() {
        mutableState.value = NewSessionUiState.Loading
        loadProjects()
    }

    /** Creates the session and stores [text] for the session route. */
    fun send(text: String) {
        val ready = (mutableState.value as? NewSessionUiState.Ready) ?: return
        if (ready.starting || text.isBlank()) return
        mutableState.value = ready.copy(starting = true, startError = null)
        viewModelScope.launch(io) {
            val result = startSafely(ready.project.id)
            when (result) {
                is StartResult.Started -> {
                    pending.put(result.sessionId, text)
                    mutableState.value = ready.copy(starting = false)
                    opened.trySend(result.sessionId)
                }
                StartResult.NoKey -> setStartError(ready, NewSessionStartError.NoKey)
                StartResult.NoModels -> setStartError(ready, NewSessionStartError.NoModels)
                StartResult.Failed -> setStartError(ready, NewSessionStartError.Failed)
            }
        }
    }

    private fun loadProjects() {
        val version = ++loadVersion
        viewModelScope.launch(io) {
            try {
                val sorted = projects.projects().sortedByDescending(ProjectSummary::lastActivityAt)
                allProjects = sorted
                val chosen =
                    requestedProjectId?.let { id -> sorted.firstOrNull { it.id == id } }
                        ?: sorted.firstOrNull()
                if (version == loadVersion) {
                    if (chosen == null) {
                        mutableState.value = NewSessionUiState.NoProject
                    } else {
                        loadPreview(chosen)
                    }
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                if (version == loadVersion) mutableState.value = NewSessionUiState.Failed
            }
        }
    }

    private fun loadPreview(project: ProjectSummary) {
        val version = ++loadVersion
        mutableState.value = NewSessionUiState.Loading
        viewModelScope.launch(io) {
            try {
                val preview = previews.preview(project.id)
                val start = starter.preview(project.id)
                if (version == loadVersion) {
                    mutableState.value =
                        NewSessionUiState.Ready(
                            project = project,
                            preview = preview,
                            modelId = start.modelId,
                            mode = start.mode,
                            projects = allProjects,
                        )
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                if (version == loadVersion) mutableState.value = NewSessionUiState.Failed
            }
        }
    }

    @Suppress("TooGenericExceptionCaught")
    private fun startSafely(projectId: String): StartResult =
        try {
            starter.start(projectId)
        } catch (_: Exception) {
            StartResult.Failed
        }

    private fun setStartError(
        ready: NewSessionUiState.Ready,
        error: NewSessionStartError,
    ) {
        mutableState.value = ready.copy(starting = false, startError = error)
    }

    companion object {
        /** Builds the model for the route. */
        fun factory(
            projects: ProjectsSource,
            previews: NewSessionGateway,
            starter: SessionStartGateway,
            pending: PendingFirstMessage,
            requestedProjectId: String? = null,
        ) = viewModelFactory {
            initializer {
                NewSessionViewModel(
                    projects,
                    previews,
                    starter,
                    pending,
                    requestedProjectId,
                )
            }
        }
    }
}
