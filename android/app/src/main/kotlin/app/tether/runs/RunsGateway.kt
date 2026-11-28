package app.tether.runs

import app.tether.core.generated.ExecutorView
import app.tether.core.generated.RunHistoryView
import app.tether.core.generated.RunOutcomeView
import app.tether.core.generated.SessionRuntime
import app.tether.session.servers.ServerProcessRegistry
import app.tether.session.tools.RunExecutor

/** The local run history the Runs screen needs, without exposing UniFFI types to its UI. */
data class LocalRun(
    val callId: String,
    val sessionId: String,
    val sessionTitle: String,
    val command: String,
    val executor: RunExecutor?,
    val outcome: RunOutcome,
    val exitCode: Int?,
    val durationMs: Long?,
    val startedAtMs: Long,
    val background: Boolean,
)

/** The terminal result of one local run. */
enum class RunOutcome {
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    OutOfMemory,
    Denied,
    Background,
    Unknown,
}

/** A blocking source for the local run history. Callers run it off the main thread. */
interface RunsGateway {
    /** The newest local runs for [projectId], at most [limit]. */
    fun localRuns(
        projectId: String,
        limit: Int,
    ): List<LocalRun>
}

/** The processes that open sessions run now, and the way to stop one. Callers run it off main. */
interface RunsProcesses {
    /** The running background processes of every open session. */
    fun open(): List<OpenProcess>

    /** Stops one process of an open session. */
    fun stop(
        sessionId: String,
        processId: String,
    )
}

/** The real source: the registry of the open session connections. */
object RegistryRunsProcesses : RunsProcesses {
    override fun open(): List<OpenProcess> =
        ServerProcessRegistry.running().map { registered ->
            OpenProcess(
                sessionId = registered.sessionId,
                processId = registered.process.id,
                command = registered.process.command,
                startedAtMs = registered.process.startedAtMs.toLong(),
            )
        }

    override fun stop(
        sessionId: String,
        processId: String,
    ) = ServerProcessRegistry.stopProcess(sessionId, processId)
}

/** The local history call failed without exposing core details to the UI. */
class RunsCallFailed(
    val failure: RunsFailure,
) : Exception(failure.name)

/** Safe categories for a local history failure. */
enum class RunsFailure {
    ProjectNotFound,
    Unavailable,
    Unknown,
}

/** The real local-runs gateway over the lazy session runtime. */
class CoreRunsGateway(
    private val runtime: Lazy<SessionRuntime>,
) : RunsGateway {
    @Suppress("SwallowedException")
    override fun localRuns(
        projectId: String,
        limit: Int,
    ): List<LocalRun> =
        try {
            runtime.value.listRuns(projectId, limit.toUInt()).map(RunHistoryView::toLocalRun)
        } catch (error: app.tether.core.generated.SessionRuntimeException.NotFound) {
            throw RunsCallFailed(RunsFailure.ProjectNotFound)
        } catch (_: app.tether.core.generated.SessionRuntimeException) {
            throw RunsCallFailed(RunsFailure.Unavailable)
        }
}

private fun RunHistoryView.toLocalRun(): LocalRun =
    LocalRun(
        callId = callId,
        sessionId = sessionId,
        sessionTitle = sessionTitle,
        command = command,
        executor = executor.toRunExecutor(),
        outcome = outcome.toRunOutcome(),
        exitCode = exitCode,
        durationMs = durationMs?.toLong(),
        startedAtMs = startedAtMs,
        background = background,
    )

private fun ExecutorView?.toRunExecutor(): RunExecutor? =
    when (this) {
        ExecutorView.SANDBOX -> RunExecutor.Sandbox
        ExecutorView.LINUX -> RunExecutor.Linux
        ExecutorView.CI -> RunExecutor.Ci
        null -> null
    }

private fun RunOutcomeView.toRunOutcome(): RunOutcome =
    when (this) {
        RunOutcomeView.SUCCEEDED -> RunOutcome.Succeeded
        RunOutcomeView.FAILED -> RunOutcome.Failed
        RunOutcomeView.TIMED_OUT -> RunOutcome.TimedOut
        RunOutcomeView.CANCELLED -> RunOutcome.Cancelled
        RunOutcomeView.OUT_OF_MEMORY -> RunOutcome.OutOfMemory
        RunOutcomeView.DENIED -> RunOutcome.Denied
        RunOutcomeView.BACKGROUND -> RunOutcome.Background
        RunOutcomeView.UNKNOWN -> RunOutcome.Unknown
    }
