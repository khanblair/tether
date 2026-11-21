package app.tether.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.session.SessionStarter
import app.tether.session.StartResult
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.receiveAsFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** Where the New session button stands. [Ready] shows no block. */
enum class StartPhase {
    /** No start is running, and none has failed. */
    Ready,

    /** A session is being created. The button shows loading and ignores taps. */
    Starting,

    /** No provider has a saved key. */
    NoKey,

    /** The provider has no saved model list. */
    NoModels,

    /** The core could not create the session. */
    Failed,
}

/**
 * What the project home shows. [project] is null when the id is unknown, and then the screen says
 * so. [sessions] are the project's sessions, newest first. [lastSession] is the most recent one.
 */
@Immutable
data class ProjectHomeUiState(
    val loading: Boolean = true,
    val project: ProjectSummary? = null,
    val sessions: List<SessionSummary> = emptyList(),
    val lastSession: SessionSummary? = null,
    val start: StartPhase = StartPhase.Ready,
)

/**
 * State holder for one project's home. It reads the project and its sessions on [io]. Starting a
 * session runs the [SessionStarter] on [io] too, and each new session id is sent once on
 * [openedSessions] for the screen to open.
 */
class ProjectHomeViewModel(
    private val projectId: String,
    private val projects: ProjectsSource,
    private val sessions: SessionsSource,
    private val starter: SessionStarter,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ProjectHomeUiState())
    private val opened = Channel<String>(Channel.BUFFERED)

    /** The project home state. */
    val state: StateFlow<ProjectHomeUiState> = mutableState.asStateFlow()

    /** Each session id the user started, once. The screen opens it. */
    val openedSessions: Flow<String> = opened.receiveAsFlow()

    init {
        refresh()
    }

    /** Reads the project and its sessions again. Call it when the screen resumes. */
    fun refresh() {
        viewModelScope.launch(io) {
            val project = projects.projects().firstOrNull { it.id == projectId }
            val all = sessions.sessions()
            mutableState.update {
                it.copy(
                    loading = false,
                    project = project,
                    sessions =
                        all
                            .filter { session -> session.projectId == projectId }
                            .sortedByDescending(SessionSummary::updatedAt),
                    lastSession = project?.let { found -> latestSessionForProject(found, all) },
                )
            }
        }
    }

    /** Starts a session in this project. A second tap while one starts does nothing. */
    fun startSession() {
        if (mutableState.value.start == StartPhase.Starting) return
        mutableState.update { it.copy(start = StartPhase.Starting) }
        viewModelScope.launch(io) {
            val phase =
                when (val result = starter.start(projectId)) {
                    is StartResult.Started -> {
                        opened.send(result.sessionId)
                        StartPhase.Ready
                    }
                    StartResult.NoKey -> StartPhase.NoKey
                    StartResult.NoModels -> StartPhase.NoModels
                    StartResult.Failed -> StartPhase.Failed
                }
            mutableState.update { it.copy(start = phase) }
        }
    }

    companion object {
        /** Builds the project home model for [projectId]. */
        fun factory(
            projectId: String,
            projects: ProjectsSource,
            sessions: SessionsSource,
            starter: SessionStarter,
        ) = viewModelFactory {
            initializer { ProjectHomeViewModel(projectId, projects, sessions, starter) }
        }
    }
}
