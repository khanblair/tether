package app.tether.home

import app.tether.core.generated.SessionStatusView
import app.tether.design.components.SessionState
import app.tether.session.SessionGateway
import app.tether.session.SessionInfo

/**
 * Maps a core session status to the state the lists show. Running covers every turn step, and
 * Waiting covers approval and pause. Interrupted counts as Failed, so Needs attention lists it.
 */
internal fun SessionStatusView.toSessionState(): SessionState =
    when (this) {
        SessionStatusView.IDLE -> SessionState.Idle
        SessionStatusView.REQUESTING,
        SessionStatusView.STREAMING,
        SessionStatusView.CHECKING_TOOL,
        SessionStatusView.RUNNING_TOOL,
        -> SessionState.Running
        SessionStatusView.AWAITING_APPROVAL,
        SessionStatusView.PAUSED,
        -> SessionState.Waiting
        SessionStatusView.INTERRUPTED,
        SessionStatusView.FAILED,
        -> SessionState.Failed
    }

/**
 * The projects from the core. Branch is blank and uncommitted is 0 until Git data is wired, and a
 * project is running when any of its sessions is running.
 */
class CoreProjectsSource(
    private val projects: ProjectGateway,
    private val sessions: SessionGateway,
) : ProjectsSource {
    override fun projects(): List<ProjectSummary> =
        projects.list().map { project ->
            ProjectSummary(
                id = project.id,
                name = project.name,
                branch = "",
                uncommitted = 0,
                lastActivityAt = project.lastActivityMs,
                running =
                    sessions.listSessions(project.id).any {
                        it.status.toSessionState() == SessionState.Running
                    },
            )
        }
}

/**
 * The sessions of every project, from the core, each with its own last change time. Sessions keep
 * the core's order within a project.
 */
class CoreSessionsSource(
    private val projects: ProjectGateway,
    private val sessions: SessionGateway,
) : SessionsSource {
    override fun sessions(): List<SessionSummary> =
        projects.list().flatMap { project ->
            sessions.listSessions(project.id).map { it.toSummary(project) }
        }

    override fun archivedSessions(): List<SessionSummary> =
        projects.list().flatMap { project ->
            sessions.listArchived(project.id).map { it.toSummary(project) }
        }

    private fun SessionInfo.toSummary(project: ProjectInfo): SessionSummary =
        SessionSummary(
            id = id,
            title = title,
            projectName = project.name,
            state = status.toSessionState(),
            updatedAt = updatedAtMs,
            projectId = project.id,
        )
}
