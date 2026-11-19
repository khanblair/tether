package app.tether.home

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.components.BadgeTone
import app.tether.design.components.StatusBadge

/**
 * The summary line of a project row: the uncommitted change count, or "Clean", then the time of
 * the last activity. [now] is the time the relative time is taken from.
 */
@Composable
internal fun projectSummary(
    project: ProjectSummary,
    now: Long,
): String {
    val ago = relativeTime(now, project.lastActivityAt).text()
    return if (project.uncommitted > 0) {
        pluralStringResource(
            R.plurals.home_project_dirty,
            project.uncommitted,
            project.uncommitted,
            ago,
        )
    } else {
        stringResource(R.string.home_project_clean, ago)
    }
}

/** The badge that marks a project with a session running. */
@Composable
internal fun RunningProjectBadge() {
    StatusBadge(
        label = stringResource(R.string.home_running_badge),
        tone = BadgeTone.Tonal,
        icon = TetherIcons.Loader,
    )
}
