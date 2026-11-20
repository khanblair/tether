package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** A project's branch, working changes, most recent session, and executor choice. */
@Composable
fun ProjectHomeScreen(
    project: ProjectSummary?,
    lastSession: SessionSummary?,
    onBack: () -> Unit,
    onNewSession: () -> Unit,
) {
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = project?.name ?: stringResource(R.string.route_project_home_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
        bottomBar = {
            if (project != null) {
                PrimaryButton(
                    label = stringResource(R.string.project_home_new_session),
                    onClick = onNewSession,
                    modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4),
                )
            }
        },
    ) { padding ->
        Column(
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
            modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4),
        ) {
            if (project == null) {
                Text(
                    stringResource(R.string.project_home_missing),
                    color = TetherTheme.colors.text2,
                )
            } else {
                ProjectFact(
                    label = stringResource(R.string.project_home_branch),
                    value =
                        project.branch.ifBlank {
                            stringResource(
                                R.string.project_home_no_branch,
                            )
                        },
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
                    value = lastSession?.title ?: stringResource(R.string.project_home_no_session),
                )
                ProjectFact(
                    label = stringResource(R.string.project_home_executor),
                    value = executorName(project.executor),
                )
            }
        }
    }
}

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
