package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SessionRow
import app.tether.design.components.SessionState
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace
import app.tether.providers.ConnectionMessage
import app.tether.shell.BackButton

/**
 * A project's branch, working changes, last session, executor, and session list. The bottom
 * button starts a session, and a block above it says what to do when a session cannot start.
 */
@Composable
fun ProjectHomeScreen(
    state: ProjectHomeUiState,
    onBack: () -> Unit,
    onOpenSession: (String) -> Unit,
    onNewSession: () -> Unit,
    onTestConnection: () -> Unit,
    onOpenProviders: () -> Unit,
    onExport: () -> Unit,
    onOpenChanges: () -> Unit,
    onOpenFiles: () -> Unit = {},
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = state.project?.name ?: stringResource(R.string.route_project_home_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
        bottomBar = {
            if (state.project != null) {
                Column(
                    verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
                    modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4),
                ) {
                    StartBlock(
                        start = state.start,
                        problem = state.testProblem,
                        onTestConnection = onTestConnection,
                        onOpenProviders = onOpenProviders,
                        onNewSession = onNewSession,
                    )
                    PrimaryButton(
                        label = stringResource(R.string.project_home_new_session),
                        onClick = onNewSession,
                        loading = state.start == StartPhase.Starting,
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
        },
    ) { padding ->
        ProjectHomeBody(
            state = state,
            onOpenSession = onOpenSession,
            onOpenFiles = onOpenFiles,
            onExport = onExport,
            onOpenChanges = onOpenChanges,
            modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4),
        )
    }
}

@Composable
private fun ProjectHomeBody(
    state: ProjectHomeUiState,
    onOpenSession: (String) -> Unit,
    onOpenFiles: () -> Unit,
    onExport: () -> Unit,
    onOpenChanges: () -> Unit,
    modifier: Modifier,
) {
    val project = state.project
    if (state.loading) return
    if (project == null) {
        Text(
            stringResource(R.string.project_home_missing),
            color = TetherTheme.colors.text2,
            modifier = modifier,
        )
        return
    }
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space4), modifier = modifier) {
        ProjectFact(
            label = stringResource(R.string.project_home_branch),
            value = project.branch.ifBlank { stringResource(R.string.project_home_no_branch) },
        )
        ProjectFact(
            label = stringResource(R.string.project_home_changes),
            value =
                pluralStringResource(
                    R.plurals.project_home_change_count,
                    project.uncommitted,
                    project.uncommitted,
                ),
        )
        ProjectFact(
            label = stringResource(R.string.project_home_last_session),
            value = state.lastSession?.title ?: stringResource(R.string.project_home_no_session),
        )
        ProjectFact(
            label = stringResource(R.string.project_home_executor),
            value = executorName(project.executor),
        )
        ProjectLinks(onOpenFiles = onOpenFiles, onOpenChanges = onOpenChanges, onExport = onExport)
        SessionList(sessions = state.sessions, onOpenSession = onOpenSession)
    }
}

/** The rows that open the project's files and export it. The agent reads the same files. */
@Composable
private fun ProjectLinks(
    onOpenFiles: () -> Unit,
    onOpenChanges: () -> Unit,
    onExport: () -> Unit,
) {
    ListGroup {
        ListRow(
            title = stringResource(R.string.files_home_title),
            subtitle = stringResource(R.string.files_home_body),
            leading = {
                TetherIcon(
                    icon = TetherIcons.Folder,
                    contentDescription = null,
                    tint = TetherTheme.colors.text2,
                )
            },
            trailing = {
                TetherIcon(
                    icon = TetherIcons.ChevronRight,
                    contentDescription = null,
                    tint = TetherTheme.colors.text3,
                )
            },
            onClick = onOpenFiles,
        )
        HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
        ListRow(
            title = stringResource(R.string.git_home_title),
            subtitle = stringResource(R.string.git_home_body),
            leading = {
                TetherIcon(
                    icon = TetherIcons.GitBranch,
                    contentDescription = null,
                    tint = TetherTheme.colors.text2,
                )
            },
            trailing = {
                TetherIcon(
                    icon = TetherIcons.ChevronRight,
                    contentDescription = null,
                    tint = TetherTheme.colors.text3,
                )
            },
            onClick = onOpenChanges,
        )
        HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
        ListRow(
            title = stringResource(R.string.archive_home_export_title),
            subtitle = stringResource(R.string.archive_home_export_body),
            leading = {
                TetherIcon(
                    icon = TetherIcons.Upload,
                    contentDescription = null,
                    tint = TetherTheme.colors.text2,
                )
            },
            onClick = onExport,
        )
    }
}

/** The sessions of the project, newest first. An empty list says so in one sentence. */
@Composable
private fun SessionList(
    sessions: List<SessionSummary>,
    onOpenSession: (String) -> Unit,
) {
    Text(
        text = stringResource(R.string.project_home_sessions_title),
        style = MaterialTheme.typography.labelMedium,
        color = TetherTheme.colors.text2,
    )
    if (sessions.isEmpty()) {
        Text(
            text = stringResource(R.string.project_home_sessions_empty),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        return
    }
    ListGroup {
        sessions.forEachIndexed { index, session ->
            if (index > 0) {
                HorizontalDivider(
                    thickness = TetherBorder.hairline,
                    color = TetherTheme.colors.line,
                )
            }
            SessionRow(
                title = session.title,
                subtitle = sessionSubtitle(session.state),
                state = session.state,
                onClick = { onOpenSession(session.id) },
            )
        }
    }
}

/**
 * The block above the New session button. Each start result has one block. A missing model list
 * or a failed connection test offers Test connection, with a text link to Providers.
 */
@Composable
private fun StartBlock(
    start: StartPhase,
    problem: ConnectionMessage?,
    onTestConnection: () -> Unit,
    onOpenProviders: () -> Unit,
    onNewSession: () -> Unit,
) {
    when (start) {
        StartPhase.Ready, StartPhase.Starting -> Unit
        StartPhase.NoKey ->
            InlineBanner(
                text = stringResource(R.string.project_home_no_key),
                tone = BannerTone.Warning,
                actionLabel = stringResource(R.string.project_home_add_key),
                onAction = onOpenProviders,
            )
        StartPhase.Testing ->
            InlineBanner(
                text = stringResource(R.string.project_home_testing),
                tone = BannerTone.Info,
            )
        StartPhase.NoModels ->
            TestBlock(
                text = stringResource(R.string.project_home_no_models),
                title = null,
                tone = BannerTone.Warning,
                onTestConnection = onTestConnection,
                onOpenProviders = onOpenProviders,
            )
        StartPhase.TestFailed ->
            TestBlock(
                text =
                    problem?.let { stringResource(it.body) }
                        ?: stringResource(R.string.project_home_test_failed),
                title = problem?.let { stringResource(it.title) },
                tone = problem?.tone ?: BannerTone.Danger,
                onTestConnection = onTestConnection,
                onOpenProviders = onOpenProviders,
            )
        StartPhase.Failed ->
            InlineBanner(
                text = stringResource(R.string.project_home_start_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.project_home_try_again),
                onAction = onNewSession,
            )
    }
}

/** A banner with Test connection, and a text link to Providers under it. */
@Composable
private fun TestBlock(
    text: String,
    title: String?,
    tone: BannerTone,
    onTestConnection: () -> Unit,
    onOpenProviders: () -> Unit,
) {
    InlineBanner(
        text = text,
        title = title,
        tone = tone,
        actionLabel = stringResource(R.string.project_home_test_connection),
        onAction = onTestConnection,
    )
    TetherTextButton(
        label = stringResource(R.string.project_home_open_providers),
        onClick = onOpenProviders,
    )
}

@Composable
private fun sessionSubtitle(state: SessionState): String =
    stringResource(
        when (state) {
            SessionState.Running -> R.string.drawer_state_running
            SessionState.Waiting -> R.string.drawer_state_waiting
            SessionState.Failed -> R.string.project_session_failed
            SessionState.Done -> R.string.project_session_done
            SessionState.Idle -> R.string.project_session_idle
        },
    )

@Composable
private fun ProjectFact(
    label: String,
    value: String,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = TetherTheme.colors.text2)
        Text(value, style = MaterialTheme.typography.bodyLarge, color = TetherTheme.colors.text)
    }
}

@Composable
private fun executorName(executor: ProjectExecutor?): String =
    stringResource(
        when (executor) {
            ProjectExecutor.Sandbox -> R.string.project_home_sandbox
            ProjectExecutor.Ci -> R.string.project_home_ci
            ProjectExecutor.Linux -> R.string.project_home_linux
            null -> R.string.project_home_no_executor
        },
    )

/** Picks the newest saved session for this project's stable id. */
fun latestSessionForProject(
    project: ProjectSummary,
    sessions: List<SessionSummary>,
): SessionSummary? =
    sessions
        .filter {
            it.projectId == project.id || (it.projectId == null && it.projectName == project.name)
        }.maxByOrNull(SessionSummary::updatedAt)
