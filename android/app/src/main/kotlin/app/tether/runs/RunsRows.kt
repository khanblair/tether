package app.tether.runs

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.home.relativeTime
import app.tether.home.text
import app.tether.session.tools.ExecutorBadge
import app.tether.session.tools.RunExecutor
import app.tether.settings.executors.CiRunRow
import app.tether.settings.executors.CiRunState
import app.tether.settings.executors.CiRunsContent
import app.tether.settings.executors.DurationParts
import app.tether.settings.executors.durationParts
import app.tether.settings.executors.labelRes
import app.tether.settings.executors.text as executorText

private const val SKELETON_ROWS = 3
private const val MILLISECONDS_PER_SECOND = 1_000L

/** The scrollable list and its loading, empty, error, and no-project states. */
@Composable
fun RunsList(
    content: RunsContent,
    onRetry: () -> Unit,
    onOpenLocal: (String) -> Unit,
    onOpenCi: (String) -> Unit,
    modifier: Modifier = Modifier,
    ciContent: CiRunsContent? = null,
    failureText: String? = null,
) {
    if (ciContent != null) {
        CiRunsListContent(
            content = ciContent,
            onRetry = onRetry,
            onOpenCi = onOpenCi,
            onOpenLocal = onOpenLocal,
            modifier = modifier,
            failureText = failureText,
        )
        return
    }
    when (content) {
        RunsContent.Loading -> LoadingRuns(modifier)
        RunsContent.Empty ->
            EmptyRuns(modifier)
        RunsContent.NoProject ->
            MessageRuns(
                text = stringResource(R.string.runs_no_project),
                modifier = modifier,
            )
        is RunsContent.Failed ->
            MessageRuns(
                text = failureText ?: stringResource(R.string.runs_failed),
                onRetry = onRetry,
                modifier = modifier,
            )
        is RunsContent.Runs -> RunRowsList(content.rows, onOpenLocal, onOpenCi, modifier)
    }
}

@Composable
private fun CiRunsListContent(
    content: CiRunsContent,
    onRetry: () -> Unit,
    onOpenLocal: (String) -> Unit,
    onOpenCi: (String) -> Unit,
    modifier: Modifier,
    failureText: String?,
) {
    when (content) {
        CiRunsContent.Loading -> LoadingRuns(modifier)
        CiRunsContent.NoProject ->
            MessageRuns(
                text = stringResource(R.string.runs_no_project),
                modifier = modifier,
            )
        is CiRunsContent.NotReady ->
            MessageRuns(
                text = content.readiness.executorText(),
                modifier = modifier,
            )
        is CiRunsContent.Failed ->
            MessageRuns(
                text = failureText ?: content.failure.executorText(),
                onRetry = onRetry,
                modifier = modifier,
            )
        CiRunsContent.Empty -> EmptyRuns(modifier)
        is CiRunsContent.Runs ->
            RunRowsList(
                rows = content.rows.map(CiRunRow::toRunRow),
                onOpenLocal = onOpenLocal,
                onOpenCi = onOpenCi,
                modifier = modifier,
            )
    }
}

@Composable
private fun LoadingRuns(modifier: Modifier) {
    LazyColumn(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        items(SKELETON_ROWS) { SkeletonRow() }
    }
}

@Composable
private fun RunRowsList(
    rows: List<RunRow>,
    onOpenLocal: (String) -> Unit,
    onOpenCi: (String) -> Unit,
    modifier: Modifier,
) {
    LazyColumn(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        items(items = rows, key = { row -> row.key() }) { row ->
            RunsRunRow(row = row, onOpenLocal = onOpenLocal, onOpenCi = onOpenCi)
        }
    }
}

/** A row in the combined local and GitHub list. */
@Composable
fun RunsRunRow(
    row: RunRow,
    onOpenLocal: (String) -> Unit,
    onOpenCi: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val state = row.sessionState()
    val title = row.title()
    val subtitle = row.subtitle()
    val executor = row.executor()
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.topBar)
                .clickable(role = Role.Button) {
                    when (row) {
                        is RunRow.Local -> onOpenLocal(row.run.sessionId)
                        is RunRow.Ci -> onOpenCi(row.htmlUrl)
                    }
                }.padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        StatusMark(state = state)
        Spacer(Modifier.width(TetherSpace.space3))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = TetherTheme.mono,
                color = TetherTheme.colors.text,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text2,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
        if (executor != null) {
            Spacer(Modifier.width(TetherSpace.space3))
            ExecutorBadge(executor = executor)
        }
    }
}

@Composable
private fun MessageRuns(
    text: String,
    modifier: Modifier,
    onRetry: (() -> Unit)? = null,
) {
    LazyColumn(modifier = modifier.fillMaxWidth()) {
        item {
            ListGroup {
                Column(
                    modifier = Modifier.padding(TetherSpace.space4),
                    verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
                ) {
                    Text(
                        text = text,
                        style = MaterialTheme.typography.bodyMedium,
                        color = TetherTheme.colors.text2,
                    )
                    if (onRetry != null) {
                        androidx.compose.material3.TextButton(onClick = onRetry) {
                            Text(stringResource(R.string.runs_try_again))
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun EmptyRuns(modifier: Modifier) {
    LazyColumn(modifier = modifier.fillMaxWidth()) {
        item {
            Text(
                text = stringResource(R.string.runs_empty),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
                modifier = Modifier.padding(TetherSpace.space4),
            )
        }
    }
}

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

@Composable
private fun RunRow.title(): String =
    when (this) {
        is RunRow.Local -> run.command
        is RunRow.Ci -> stringResource(R.string.runs_workflow_branch, workflow, branch)
    }

@Composable
private fun RunRow.subtitle(): String =
    when (this) {
        is RunRow.Local -> localMeta(run)
        is RunRow.Ci -> ciMeta(this)
    }

private fun RunRow.executor(): RunExecutor? =
    when (this) {
        is RunRow.Local -> run.executor
        is RunRow.Ci -> RunExecutor.Ci
    }

private fun RunRow.sessionState(): SessionState =
    when (this) {
        is RunRow.Local -> run.sessionState()
        is RunRow.Ci -> state.sessionState()
    }

private fun RunRow.key(): String =
    when (this) {
        is RunRow.Local -> "local-${run.callId}"
        is RunRow.Ci -> "ci-$id"
    }

@Composable
private fun localMeta(run: LocalRun): String {
    val age = relativeTime(System.currentTimeMillis(), run.startedAtMs).text()
    val duration = run.durationMs?.let { durationText(it / MILLISECONDS_PER_SECOND) }
    return when (metaKind(run)) {
        RunMetaKind.Background -> stringResource(R.string.runs_background_meta, run.command, age)
        RunMetaKind.ExitAndDuration ->
            stringResource(
                R.string.runs_exit_duration_time,
                run.exitCode ?: 0,
                duration.orEmpty(),
                age,
            )
        RunMetaKind.DurationOnly ->
            stringResource(R.string.runs_duration_time, duration.orEmpty(), age)
        RunMetaKind.Outcome -> stringResource(R.string.runs_outcome_time, outcomeText(run), age)
    }
}

@Composable
private fun ciMeta(row: RunRow.Ci): String {
    val state = stringResource(row.state.labelRes())
    val duration = durationText(row.durationSeconds)
    val started = row.startedAtMs?.let { relativeTime(System.currentTimeMillis(), it).text() }
    return if (started == null) {
        stringResource(R.string.runs_ci_meta, state, duration)
    } else {
        stringResource(R.string.runs_ci_meta_time, state, duration, started)
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

@Composable
private fun durationText(seconds: Long): String {
    val parts: DurationParts = durationParts(seconds)
    return when {
        parts.hours > 0L ->
            stringResource(R.string.executors_duration_hours, parts.hours, parts.minutes)
        parts.minutes > 0L ->
            stringResource(R.string.executors_duration_minutes, parts.minutes, parts.seconds)
        else -> stringResource(R.string.executors_duration_seconds, parts.seconds)
    }
}

private fun CiRunState.sessionState(): SessionState =
    when (this) {
        CiRunState.Queued -> SessionState.Waiting
        CiRunState.Running -> SessionState.Running
        CiRunState.Succeeded -> SessionState.Done
        CiRunState.Failed -> SessionState.Failed
        CiRunState.Cancelled, CiRunState.Finished -> SessionState.Idle
    }
