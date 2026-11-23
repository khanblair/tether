package app.tether.settings.executors

import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R

/** The sentence for a failure. Every value is listed, so a new one needs words to build. */
@StringRes
internal fun ExecutorsFailure.messageRes(): Int =
    when (this) {
        ExecutorsFailure.NotConfigured -> R.string.executors_error_not_configured
        ExecutorsFailure.Unavailable -> R.string.executors_error_unavailable
        ExecutorsFailure.ProjectNotFound -> R.string.executors_error_project_not_found
        ExecutorsFailure.NoRepository -> R.string.executors_error_no_repository
        ExecutorsFailure.NotSignedIn -> R.string.executors_error_not_signed_in
        ExecutorsFailure.NoRemote -> R.string.executors_error_no_remote
        ExecutorsFailure.GithubError -> R.string.executors_error_github
        ExecutorsFailure.VerificationFailed -> R.string.executors_error_verification
        ExecutorsFailure.Unknown -> R.string.executors_error_unknown
    }

/** The sentence for a CI readiness value. */
@StringRes
internal fun CiReadiness.messageRes(): Int =
    when (this) {
        CiReadiness.Ready -> R.string.executors_ci_ready
        CiReadiness.NotSignedIn -> R.string.executors_ci_not_signed_in
        CiReadiness.NoRemote -> R.string.executors_ci_no_remote
        CiReadiness.WorkflowMissing -> R.string.executors_ci_workflow_missing
        CiReadiness.WorkflowOutdated -> R.string.executors_ci_workflow_outdated
        CiReadiness.Unavailable -> R.string.executors_ci_unavailable
    }

/** The word for a run state, shown next to its icon. */
@StringRes
internal fun CiRunState.labelRes(): Int =
    when (this) {
        CiRunState.Queued -> R.string.executors_run_queued
        CiRunState.Running -> R.string.executors_run_running
        CiRunState.Succeeded -> R.string.executors_run_succeeded
        CiRunState.Failed -> R.string.executors_run_failed
        CiRunState.Cancelled -> R.string.executors_run_cancelled
        CiRunState.Finished -> R.string.executors_run_finished
    }

/** The sentence for a failure, as shown in a row or banner. */
@Composable
internal fun ExecutorsFailure.text(): String = stringResource(messageRes())

/** The sentence for a CI readiness value. */
@Composable
internal fun CiReadiness.text(): String = stringResource(messageRes())

/** The reason an executor cannot be offered in this build. */
@Composable
internal fun LinuxUnavailable.text(): String =
    stringResource(
        when (this) {
            LinuxUnavailable.ThisBuild -> R.string.executors_na_linux_build
            LinuxUnavailable.NotConfigured -> R.string.executors_na_config
        },
    )
