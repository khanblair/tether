package app.tether.settings.executors

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListRow
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.home.relativeTime
import app.tether.home.text

/** Placeholder rows shown while the first load runs. */
private const val SKELETON_ROWS = 3

/**
 * The recent CI runs of one project. [runs] is null while the first load runs, and shows
 * placeholder rows. An empty list shows one sentence. Every state is a lazy list, so a pull to
 * refresh works on it. Each row opens the run's page through [onOpen], which gets the GitHub URL.
 */
@Composable
fun CiRunsList(
    runs: List<CiRunRow>?,
    onOpen: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space0_5),
    ) {
        when {
            runs == null -> items(SKELETON_ROWS) { SkeletonRow() }
            runs.isEmpty() -> item(key = "empty") { EmptyText() }
            else -> {
                item(key = "count") { CountLine(runs.size) }
                items(items = runs, key = { it.id }) { run ->
                    RunRow(run = run, onOpen = { onOpen(run.htmlUrl) })
                }
            }
        }
    }
}

/** The count line above the rows. */
@Composable
private fun CountLine(count: Int) {
    Text(
        text = pluralStringResource(R.plurals.executors_runs_count, count, count),
        style = MaterialTheme.typography.labelMedium,
        color = TetherTheme.colors.text3,
        modifier = Modifier.padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
    )
}

/** The sentence for a project with no runs yet. */
@Composable
private fun EmptyText() {
    Text(
        text = stringResource(R.string.executors_runs_empty),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(TetherSpace.space4),
    )
}

/** One run: its state icon and word, the branch, how long it took, and when it started. */
@Composable
private fun RunRow(
    run: CiRunRow,
    onOpen: () -> Unit,
) {
    val started = run.startedAtMs?.let { relativeTime(System.currentTimeMillis(), it).text() }
    val duration = durationText(run.durationSeconds)
    val state = stringResource(run.state.labelRes())
    val summary =
        if (started == null) {
            stringResource(R.string.executors_run_summary_no_time, state, run.branch, duration)
        } else {
            stringResource(R.string.executors_run_summary, state, run.branch, duration, started)
        }
    ListRow(
        title = run.title,
        subtitle = summary,
        leading = { RunStateMark(run.state) },
        onClick = onOpen,
    )
}

/** The icon for a run state. Each icon has a word beside it in the summary, so it is decorative. */
@Composable
private fun RunStateMark(state: CiRunState) {
    when (state) {
        CiRunState.Cancelled ->
            TetherIcon(
                icon = TetherIcons.X,
                contentDescription = null,
                tint = TetherTheme.colors.text3,
            )
        else -> StatusMark(state = state.sessionState())
    }
}

/** The shared status icon for each run state. */
private fun CiRunState.sessionState(): SessionState =
    when (this) {
        CiRunState.Queued -> SessionState.Waiting
        CiRunState.Running -> SessionState.Running
        CiRunState.Succeeded -> SessionState.Done
        CiRunState.Failed -> SessionState.Failed
        CiRunState.Cancelled -> SessionState.Idle
        CiRunState.Finished -> SessionState.Idle
    }

/** A run's length in hours and minutes, minutes and seconds, or seconds alone. */
@Composable
private fun durationText(seconds: Long): String {
    val parts = durationParts(seconds)
    return when {
        parts.hours > 0L ->
            stringResource(
                R.string.executors_duration_hours,
                parts.hours,
                parts.minutes,
            )
        parts.minutes > 0L ->
            stringResource(
                R.string.executors_duration_minutes,
                parts.minutes,
                parts.seconds,
            )
        else -> stringResource(R.string.executors_duration_seconds, parts.seconds)
    }
}

/** A placeholder row with the same height as a real row. */
@Composable
private fun SkeletonRow() {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space4)
                .height(TetherSize.touchMin)
                .background(
                    TetherTheme.colors.surfaceSunken,
                    RoundedCornerShape(TetherRadius.sm),
                ),
    )
}
