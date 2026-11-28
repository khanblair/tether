package app.tether.runs

import androidx.compose.runtime.Immutable
import app.tether.design.components.SessionState
import app.tether.session.tools.RunExecutor
import app.tether.settings.executors.CiRunRow
import app.tether.settings.executors.CiRunState
import app.tether.settings.executors.CiRunsContent
import kotlin.math.abs

/** A background process that is alive in an open session, matched to one run row. */
data class LiveProcess(
    val sessionId: String,
    val processId: String,
)

/** One process that an open session runs now, as the registry reports it. */
data class OpenProcess(
    val sessionId: String,
    val processId: String,
    val command: String,
    val startedAtMs: Long,
)

/** What a local row shows: its process still runs, its process is gone, or it ended by itself. */
enum class LocalRunStatus {
    Live,
    Stopped,
    Finished,
}

/** Live when a process is matched, Stopped for a background run without one, else Finished. */
fun RunRow.Local.status(): LocalRunStatus =
    when {
        live != null -> LocalRunStatus.Live
        run.background || run.outcome == RunOutcome.Background -> LocalRunStatus.Stopped
        else -> LocalRunStatus.Finished
    }

/** The status mark of a local row. A background run only shows Running while its process runs. */
fun RunRow.Local.markState(): SessionState =
    when (status()) {
        LocalRunStatus.Live -> SessionState.Running
        LocalRunStatus.Stopped -> SessionState.Idle
        LocalRunStatus.Finished -> run.sessionState()
    }

/** The executor filter shown above the list. */
enum class RunsFilter {
    All,
    Sandbox,
    Linux,
    Ci,
}

/** One row in the combined local and GitHub history. */
sealed interface RunRow {
    val startedAtMs: Long?

    /**
     * A command saved by the local session runtime. [live] is the process that still runs for a
     * background command in an open session, and null for every other run.
     */
    data class Local(
        val run: LocalRun,
        val live: LiveProcess? = null,
    ) : RunRow {
        override val startedAtMs: Long = run.startedAtMs
    }

    /** A workflow run read from GitHub. */
    data class Ci(
        val id: Long,
        val workflow: String,
        val branch: String,
        val state: CiRunState,
        val durationSeconds: Long,
        override val startedAtMs: Long?,
        val htmlUrl: String,
    ) : RunRow
}

/** The local or combined list area. */
sealed interface RunsContent {
    data object Loading : RunsContent

    data object NoProject : RunsContent

    data object Empty : RunsContent

    data class Failed(
        val failure: RunsFailure,
    ) : RunsContent

    data class Runs(
        val rows: List<RunRow>,
    ) : RunsContent
}

/** Everything the Runs screen renders. */
@Immutable
data class RunsUiState(
    val projectId: String? = null,
    val filter: RunsFilter = RunsFilter.All,
    val content: RunsContent = RunsContent.Loading,
    val ciContent: CiRunsContent = CiRunsContent.Loading,
    val refreshing: Boolean = false,
)

/** The local row status and its metadata shape are pure presentation decisions. */
fun LocalRun.sessionState(): SessionState = runState(outcome, background)

/** Maps the core outcome to the shared session status mark. */
fun runState(
    outcome: RunOutcome,
    background: Boolean,
): SessionState =
    when {
        background || outcome == RunOutcome.Background -> SessionState.Running
        outcome == RunOutcome.Succeeded -> SessionState.Done
        outcome == RunOutcome.Failed ||
            outcome == RunOutcome.TimedOut ||
            outcome == RunOutcome.OutOfMemory -> SessionState.Failed
        outcome == RunOutcome.Cancelled || outcome == RunOutcome.Denied -> SessionState.Idle
        else -> SessionState.Idle
    }

/** The choices for a foreground or background metadata line. */
enum class RunMetaKind {
    Background,
    ExitAndDuration,
    DurationOnly,
    Outcome,
}

/** Chooses the metadata shape without doing resource or clock work. */
fun metaKind(run: LocalRun): RunMetaKind =
    when {
        run.background || run.outcome == RunOutcome.Background -> RunMetaKind.Background
        run.exitCode != null && run.durationMs != null -> RunMetaKind.ExitAndDuration
        run.durationMs != null -> RunMetaKind.DurationOnly
        else -> RunMetaKind.Outcome
    }

/** Merges the rows newest first and replaces local CI records with GitHub rows. */
fun mergeRuns(
    local: List<RunRow.Local>,
    ci: List<RunRow.Ci>,
): List<RunRow> =
    (local.filterNot { it.run.executor == RunExecutor.Ci } + ci)
        .sortedByDescending { it.startedAtMs ?: Long.MIN_VALUE }

internal fun CiRunRow.toRunRow(): RunRow.Ci =
    RunRow.Ci(
        id = id,
        workflow = title,
        branch = branch,
        state = state,
        durationSeconds = durationSeconds,
        startedAtMs = startedAtMs,
        htmlUrl = htmlUrl,
    )

/** The history keeps the first 500 characters of a command, so processes compare on those. */
private const val COMMAND_COMPARE_LENGTH = 500

/**
 * Pairs each background run with the process of the same session and command that is alive now.
 * One process serves one run: the newest run takes the process whose start is nearest to its own.
 * The result maps a run's call id to its process.
 */
fun matchLive(
    runs: List<LocalRun>,
    processes: List<OpenProcess>,
): Map<String, LiveProcess> {
    val free = processes.toMutableList()
    val matched = mutableMapOf<String, LiveProcess>()
    runs
        .filter { it.background || it.outcome == RunOutcome.Background }
        .sortedByDescending { it.startedAtMs }
        .forEach { run ->
            val process =
                free
                    .filter { it.sessionId == run.sessionId && it.sameCommandAs(run) }
                    .minByOrNull { abs(it.startedAtMs - run.startedAtMs) }
            if (process != null) {
                free.remove(process)
                matched[run.callId] = LiveProcess(process.sessionId, process.processId)
            }
        }
    return matched
}

private fun OpenProcess.sameCommandAs(run: LocalRun): Boolean =
    command.take(COMMAND_COMPARE_LENGTH) == run.command.take(COMMAND_COMPARE_LENGTH)
