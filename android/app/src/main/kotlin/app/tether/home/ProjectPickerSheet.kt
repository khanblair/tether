package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSpace

/** A generic project picker used by screens that need a project without changing it. */
@Composable
fun ProjectPickerSheet(
    projects: List<ProjectSummary>,
    selectedId: String?,
    onSelect: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(
        title = stringResource(R.string.new_session_project_picker_title),
        onDismiss = onDismiss,
    ) {
        if (projects.isEmpty()) {
            Column(
                modifier = Modifier.padding(TetherSpace.space4),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
            ) {
                Text(
                    text = stringResource(R.string.new_session_no_projects),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text2,
                )
            }
        } else {
            val now = remember { System.currentTimeMillis() }
            LazyColumn {
                items(projects, key = ProjectSummary::id) { project ->
                    val selected = project.id == selectedId
                    ListRow(
                        title = project.name,
                        subtitle = projectLastActivity(project, now),
                        leading = {
                            TetherIcon(
                                icon = TetherIcons.Folder,
                                contentDescription = null,
                                tint = TetherTheme.colors.text2,
                            )
                        },
                        trailing = {
                            if (selected) {
                                TetherIcon(
                                    icon = TetherIcons.CircleCheck,
                                    contentDescription =
                                        stringResource(R.string.new_session_selected),
                                    tint = TetherTheme.colors.accent,
                                )
                            }
                        },
                        onClick = { onSelect(project.id) },
                        modifier = Modifier.semantics { this.selected = selected },
                    )
                }
            }
        }
    }
}

@Composable
private fun projectLastActivity(
    project: ProjectSummary,
    now: Long,
): String =
    stringResource(
        R.string.new_session_last_active,
        relativeTime(now, project.lastActivityAt).text(),
    )
