package app.tether.settings.executors

import app.tether.core.generated.CiRuntimeException
import app.tether.core.generated.ExecutorRuntimeException

/** The core's CI reason for a missing sign-in. These strings come from the core's ci.rs. */
private const val REASON_SIGNED_OUT = "GitHub is not signed in"

/** The core's CI reason for a project with no GitHub remote. */
private const val REASON_NO_REMOTE = "project does not have a GitHub remote"

/** The core's CI reason for a missing workflow file. */
private const val REASON_WORKFLOW_MISSING = "workflow is missing"

/** The core's CI reason for an old workflow file. */
private const val REASON_WORKFLOW_OUTDATED = "workflow is outdated"

/**
 * Maps a core executor error to the failure the screens show. The `when` has no `else`, so a new
 * variant stops the build until it is mapped.
 */
fun ExecutorRuntimeException.toFailure(): ExecutorsFailure =
    when (this) {
        is ExecutorRuntimeException.InvalidConfiguration -> ExecutorsFailure.NotConfigured
        is ExecutorRuntimeException.Unavailable -> ExecutorsFailure.Unavailable
        is ExecutorRuntimeException.VerificationFailed -> ExecutorsFailure.VerificationFailed
        is ExecutorRuntimeException.ProjectNotFound -> ExecutorsFailure.ProjectNotFound
        is ExecutorRuntimeException.LinuxNotReady -> ExecutorsFailure.Unavailable
        is ExecutorRuntimeException.LinuxShellAlreadyOpen -> ExecutorsFailure.Unknown
    }

/**
 * Maps a core CI error to the failure the screens show. The GitHub detail is dropped on purpose.
 */
fun CiRuntimeException.toFailure(): ExecutorsFailure =
    when (this) {
        is CiRuntimeException.Repository -> ExecutorsFailure.NoRepository
        is CiRuntimeException.NotSignedIn -> ExecutorsFailure.NotSignedIn
        is CiRuntimeException.NoRemote -> ExecutorsFailure.NoRemote
        is CiRuntimeException.Unavailable -> ExecutorsFailure.Unavailable
        is CiRuntimeException.Github -> ExecutorsFailure.GithubError
    }

/** The failure behind an error a ViewModel caught. Other errors are [ExecutorsFailure.Unknown]. */
fun Throwable.toExecutorsFailure(): ExecutorsFailure =
    if (this is ExecutorsCallFailed) failure else ExecutorsFailure.Unknown

/**
 * CI readiness from the core's status fields. The core sends one reason per stage, so the reason
 * is checked first. The flags decide only when no known reason matches.
 */
fun ciReadinessOf(
    ready: Boolean,
    workflowPresent: Boolean,
    workflowCurrent: Boolean,
    reason: String,
): CiReadiness =
    when {
        reason == REASON_SIGNED_OUT -> CiReadiness.NotSignedIn
        reason == REASON_NO_REMOTE -> CiReadiness.NoRemote
        reason == REASON_WORKFLOW_MISSING -> CiReadiness.WorkflowMissing
        reason == REASON_WORKFLOW_OUTDATED -> CiReadiness.WorkflowOutdated
        ready && workflowPresent && workflowCurrent -> CiReadiness.Ready
        else -> CiReadiness.Unavailable
    }

/** The run states the list shows. */
enum class CiRunState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Finished,
}

/**
 * The run state from GitHub's `status` and `conclusion` values. Anything unknown is
 * [CiRunState.Finished], so an odd value never claims success or failure.
 */
fun ciRunStateOf(
    status: String,
    conclusion: String,
): CiRunState =
    when (status) {
        "queued", "waiting", "pending", "requested" -> CiRunState.Queued
        "in_progress" -> CiRunState.Running
        "completed" ->
            when (conclusion) {
                "success" -> CiRunState.Succeeded
                "failure", "timed_out", "startup_failure" -> CiRunState.Failed
                "cancelled" -> CiRunState.Cancelled
                else -> CiRunState.Finished
            }
        else -> CiRunState.Finished
    }
