package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.EllipsisVertical
import app.tether.design.FileDiff
import app.tether.design.TetherIcons
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTopBar
import app.tether.home.ProjectSummary
import app.tether.home.ProjectsSource
import app.tether.shell.BackButton
import kotlinx.coroutines.Dispatchers

/**
 * The session top bar: a menu button that opens the drawer, the title and the "project · branch"
 * subtitle, then Review changes and the session menu. Without a drawer, the bar has a back button.
 */
@Composable
internal fun SessionTopBar(
    title: String,
    subtitle: String?,
    onBack: () -> Unit,
    onOpenDrawer: (() -> Unit)?,
    onReviewChanges: (() -> Unit)?,
    onMore: (() -> Unit)?,
) {
    TetherTopBar(
        title = title,
        subtitle = subtitle,
        navigationIcon = {
            if (onOpenDrawer != null) {
                TetherIconButton(
                    icon = TetherIcons.Menu,
                    contentDescription = stringResource(R.string.home_menu_action),
                    onClick = onOpenDrawer,
                )
            } else {
                BackButton(onBack)
            }
        },
        actions = {
            onReviewChanges?.let {
                TetherIconButton(
                    icon = TetherIcons.FileDiff,
                    contentDescription = stringResource(R.string.session_review_changes),
                    onClick = it,
                )
            }
            onMore?.let {
                TetherIconButton(
                    icon = TetherIcons.EllipsisVertical,
                    contentDescription = stringResource(R.string.session_menu_action),
                    onClick = it,
                )
            }
        },
    )
}

/**
 * The subtitle under the session title: "project · branch", or the project name alone when the
 * branch is blank. It is null while the project is not loaded, so the title shows alone.
 * [format] is the localized "%1$s · %2$s" pattern.
 */
internal fun sessionSubtitle(
    projectName: String?,
    branch: String,
    format: String,
): String? =
    when {
        projectName == null -> null
        branch.isBlank() -> projectName
        else -> format.format(projectName, branch)
    }

/** The project of the session, read off the main thread. It is null until the read finishes. */
@Composable
internal fun rememberProjectSummary(
    source: ProjectsSource?,
    projectId: String?,
): ProjectSummary? {
    var project by remember(source, projectId) { mutableStateOf<ProjectSummary?>(null) }
    LaunchedEffect(source, projectId) {
        if (source != null && projectId != null) {
            project =
                sessionCall(Dispatchers.IO) {
                    source.projects().firstOrNull { it.id == projectId }
                }.getOrNull()
        }
    }
    return project
}
