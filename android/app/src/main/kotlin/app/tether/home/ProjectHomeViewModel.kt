package app.tether.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderView
import app.tether.providers.ConnectionMessage
import app.tether.providers.ProviderGateway
import app.tether.providers.callCore
import app.tether.providers.connectionMessage
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

    /** The provider has no saved model list. The user can test the connection. */
    NoModels,

    /** The connection test is running. The start is paused until it ends. */
    Testing,

    /** The connection test did not pass. The user can test again or open Providers. */
    TestFailed,

    /** The core could not create the session. */
    Failed,
}

/**
 * What the project home shows. [project] is null when the id is unknown, and then the screen says
 * so. [sessions] are the project's sessions, newest first. [lastSession] is the most recent one.
 * [testProblem] is the failure of the last connection test, when it has a known message.
 */
@Immutable
data class ProjectHomeUiState(
    val loading: Boolean = true,
    val project: ProjectSummary? = null,
    val sessions: List<SessionSummary> = emptyList(),
    val lastSession: SessionSummary? = null,
    val start: StartPhase = StartPhase.Ready,
    val testProblem: ConnectionMessage? = null,
)

/**
 * State holder for one project's home. It reads the project and its sessions on [io]. Starting a
 * session runs the [SessionStarter] on [io] too, and each new session id is sent once on
 * [openedSessions] for the screen to open. A connection test uses the [ProviderGateway] on [io]
 * for the first provider that has a key, then starts the session again when the key is accepted.
 */
class ProjectHomeViewModel(
    private val projectId: String,
    private val projects: ProjectsSource,
    private val sessions: SessionsSource,
    private val starter: SessionStarter,
    private val providers: ProviderGateway,
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

    /** Starts a session in this project. A second tap while one starts or tests does nothing. */
    fun startSession() {
        if (isBusy()) return
        mutableState.update { it.copy(start = StartPhase.Starting) }
        viewModelScope.launch(io) { applyStart(starter.start(projectId)) }
    }

    /**
     * Tests the connection of the first provider with a key, which saves its model list when the
     * provider accepts the key. An accepted key starts the session again. Only the no-models and
     * test-failed states can run it.
     */
    fun testConnection() {
        val phase = mutableState.value.start
        if (phase != StartPhase.NoModels && phase != StartPhase.TestFailed) return
        setPhase(StartPhase.Testing, null)
        viewModelScope.launch(io) { runTest() }
    }

    private fun isBusy(): Boolean =
        mutableState.value.start.let { it == StartPhase.Starting || it == StartPhase.Testing }

    private suspend fun runTest() {
        val result = callCore(io) { testFirstKeyed() }
        if (result.isFailure) {
            setPhase(StartPhase.TestFailed, null)
            return
        }
        when (val status = result.getOrNull()) {
            null -> setPhase(StartPhase.NoKey, null)
            ConnectionStatusView.OK -> applyStart(starter.start(projectId))
            else -> setPhase(StartPhase.TestFailed, connectionMessage(status))
        }
    }

    /** Tests the first provider with a key. Null when no provider has a key. */
    private fun testFirstKeyed(): ConnectionStatusView? {
        val provider = ProviderView.entries.firstOrNull { providers.hasKey(it) } ?: return null
        return providers.testConnection(provider).status
    }

    private suspend fun applyStart(result: StartResult) {
        when (result) {
            is StartResult.Started -> {
                opened.send(result.sessionId)
                setPhase(StartPhase.Ready, null)
            }
            StartResult.NoKey -> setPhase(StartPhase.NoKey, null)
            StartResult.NoModels -> setPhase(StartPhase.NoModels, null)
            StartResult.Failed -> setPhase(StartPhase.Failed, null)
        }
    }

    private fun setPhase(
        phase: StartPhase,
        problem: ConnectionMessage?,
    ) {
        mutableState.update { it.copy(start = phase, testProblem = problem) }
    }

    companion object {
        /** Builds the project home model for [projectId]. */
        fun factory(
            projectId: String,
            projects: ProjectsSource,
            sessions: SessionsSource,
            starter: SessionStarter,
            providers: ProviderGateway,
        ) = viewModelFactory {
            initializer {
                ProjectHomeViewModel(projectId, projects, sessions, starter, providers)
            }
        }
    }
}
