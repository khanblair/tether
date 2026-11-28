package app.tether.runs

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.RefreshCw
import app.tether.design.TetherIcons
import app.tether.design.TetherInputIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIconButton
import app.tether.home.relativeTime
import app.tether.home.text
import app.tether.settings.executors.DurationParts
import app.tether.settings.executors.durationParts

private const val MILLISECONDS_PER_SECOND = 1_000L

/** The Stop button of a live local row, or the Restart button of any other local row. */
@Composable
internal fun LocalRunAction(
    row: RunRow.Local,
    onStop: (RunRow.Local) -> Unit,
    onRestart: (RunRow.Local) -> Unit,
) {
    if (row.status() == LocalRunStatus.Live) {
        TetherIconButton(
            icon = TetherInputIcons.Square,
            contentDescription = stringResource(R.string.runs_stop, row.run.command),
            onClick = { onStop(row) },
            tint = TetherTheme.colors.danger,
        )
    } else {
        TetherIconButton(
            icon = TetherIcons.RefreshCw,
            contentDescription = stringResource(R.string.runs_restart, row.run.command),
            onClick = { onRestart(row) },
        )
    }
}

/** The first detail line of a local row: its status and age, or the finished-run metadata. */
@Composable
internal fun localMeta(row: RunRow.Local): String {
    val age = relativeTime(System.currentTimeMillis(), row.run.startedAtMs).text()
    return when (row.status()) {
        LocalRunStatus.Live -> stringResource(R.string.runs_meta_running, age)
        LocalRunStatus.Stopped -> stringResource(R.string.runs_meta_stopped, age)
        LocalRunStatus.Finished -> finishedMeta(row.run, age)
    }
}

@Composable
private fun finishedMeta(
    run: LocalRun,
    age: String,
): String {
    val duration = run.durationMs?.let { durationText(it / MILLISECONDS_PER_SECOND) }
    return when (metaKind(run)) {
        RunMetaKind.ExitAndDuration ->
            stringResource(
                R.string.runs_exit_duration_time,
                run.exitCode ?: 0,
                duration.orEmpty(),
                age,
            )
        RunMetaKind.DurationOnly ->
            stringResource(R.string.runs_duration_time, duration.orEmpty(), age)
        RunMetaKind.Background, RunMetaKind.Outcome ->
            stringResource(R.string.runs_outcome_time, outcomeText(run), age)
    }
}

@Composable
private fun outcomeText(run: LocalRun): String =
    stringResource(
        when (run.outcome) {
            RunOutcome.Succeeded -> R.string.runs_outcome_succeeded
            RunOutcome.Failed -> R.string.runs_outcome_failed
            RunOutcome.TimedOut -> R.string.runs_outcome_timed_out
            RunOutcome.Cancelled -> R.string.runs_outcome_cancelled
            RunOutcome.OutOfMemory -> R.string.runs_outcome_out_of_memory
            RunOutcome.Denied -> R.string.runs_outcome_denied
            RunOutcome.Background -> R.string.runs_outcome_background
            RunOutcome.Unknown -> R.string.runs_outcome_unknown
        },
    )

/** A duration text, shared by the local and the GitHub rows. */
@Composable
internal fun durationText(seconds: Long): String {
    val parts: DurationParts = durationParts(seconds)
    return when {
        parts.hours > 0L ->
            stringResource(R.string.executors_duration_hours, parts.hours, parts.minutes)
        parts.minutes > 0L ->
            stringResource(R.string.executors_duration_minutes, parts.minutes, parts.seconds)
        else -> stringResource(R.string.executors_duration_seconds, parts.seconds)
    }
}
