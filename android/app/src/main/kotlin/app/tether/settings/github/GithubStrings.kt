package app.tether.settings.github

import androidx.annotation.StringRes
import app.tether.R

/** The message resource for a failure. Screens show it; no core text ever reaches the UI. */
@StringRes
internal fun GithubFailure.messageRes(): Int =
    when (this) {
        GithubFailure.InvalidInput -> R.string.github_failure_invalid_input
        GithubFailure.NotFound -> R.string.github_failure_not_found
        GithubFailure.Unavailable -> R.string.github_failure_unavailable
        GithubFailure.NotConfigured -> R.string.github_failure_not_configured
        GithubFailure.NotSignedIn -> R.string.github_failure_not_signed_in
        GithubFailure.NoDeviceGrant -> R.string.github_failure_no_grant
        GithubFailure.Rejected -> R.string.github_failure_rejected
        GithubFailure.BadResponse -> R.string.github_failure_bad_response
        GithubFailure.Network -> R.string.github_failure_network
        GithubFailure.MissingScope -> R.string.github_failure_missing_scope
        GithubFailure.RepositoryFailed -> R.string.github_failure_repository
    }

/** The user-facing label for a personal access token kind. */
@StringRes
internal fun GithubTokenKind.labelRes(): Int =
    when (this) {
        GithubTokenKind.Classic -> R.string.github_account_token_kind_classic
        GithubTokenKind.FineGrained -> R.string.github_account_token_kind_fine_grained
    }

/** The user-facing label for repository reach. */
@StringRes
internal fun GithubRepositoryAccess.labelRes(): Int =
    when (this) {
        GithubRepositoryAccess.Yes -> R.string.github_account_repositories_yes
        GithubRepositoryAccess.No -> R.string.github_account_repositories_no
    }

/** The phase line for a clone progress report. */
@StringRes
internal fun ClonePhaseStep.labelRes(): Int =
    when (this) {
        ClonePhaseStep.Resolving -> R.string.github_clone_phase_resolving
        ClonePhaseStep.Receiving -> R.string.github_clone_phase_receiving
        ClonePhaseStep.CheckingOut -> R.string.github_clone_phase_checking_out
        ClonePhaseStep.Unknown -> R.string.github_clone_phase_unknown
    }

/** The history choice label. */
@StringRes
internal fun CloneHistory.labelRes(): Int =
    when (this) {
        CloneHistory.Auto -> R.string.github_clone_history_auto
        CloneHistory.Shallow -> R.string.github_clone_history_shallow
        CloneHistory.Full -> R.string.github_clone_history_full
    }

/** The message for an identity field problem. */
@StringRes
internal fun IdentityProblem.messageRes(isEmail: Boolean): Int =
    when (this) {
        IdentityProblem.Empty ->
            if (isEmail) {
                R.string.github_identity_email_empty
            } else {
                R.string.github_identity_name_empty
            }
        IdentityProblem.Forbidden -> R.string.github_identity_forbidden
    }
