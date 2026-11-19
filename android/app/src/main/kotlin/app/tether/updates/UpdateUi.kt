package app.tether.updates

import androidx.annotation.StringRes
import app.tether.R

/** A button on the update card. Each action maps to one call on the update controller. */
enum class UpdateAction { Check, Download, Cancel, Install, OpenSettings, OpenStore }

/**
 * What the update card shows for one state. It holds string resource ids and flags only, so a
 * JUnit test can check it without a Context.
 */
@Suppress("LongParameterList")
data class UpdateCardUi(
    @StringRes val title: Int,
    @StringRes val body: Int? = null,
    @StringRes val problem: Int? = null,
    val primary: UpdateAction? = null,
    val secondary: UpdateAction? = null,
    val showNotes: Boolean = false,
    val showSize: Boolean = false,
    val showProgress: Boolean = false,
    val showSpinner: Boolean = false,
    val success: Boolean = false,
    val failed: Boolean = false,
)

/**
 * Maps the update state to the card. Pure: the same state always gives the same card. The play
 * build never offers Download or Install; it sends the user to Google Play instead.
 */
internal fun updateCardFor(state: UpdatesUiState): UpdateCardUi =
    when (state.phase) {
        UpdatePhase.Idle,
        UpdatePhase.Checking,
        UpdatePhase.UpToDate,
        UpdatePhase.Available,
        -> checkCard(state)
        UpdatePhase.Downloading,
        UpdatePhase.Ready,
        UpdatePhase.NeedsPermission,
        UpdatePhase.Installing,
        UpdatePhase.Installed,
        UpdatePhase.Failed,
        -> installCard(state)
    }

/** The cards for checking: idle, checking, up to date, and a newer release found. */
private fun checkCard(state: UpdatesUiState): UpdateCardUi =
    when (state.phase) {
        UpdatePhase.Checking ->
            UpdateCardUi(
                title = R.string.updates_checking_title,
                showSpinner = true,
            )
        UpdatePhase.UpToDate ->
            UpdateCardUi(
                title = R.string.updates_up_to_date_title,
                body = R.string.updates_up_to_date_body,
                secondary = UpdateAction.Check,
                success = true,
            )
        UpdatePhase.Available -> availableCard(state.installsInApp)
        else ->
            UpdateCardUi(
                title = R.string.updates_idle_title,
                body = R.string.updates_idle_body,
                primary = UpdateAction.Check,
            )
    }

/** The cards for downloading, installing, and the failure. */
private fun installCard(state: UpdatesUiState): UpdateCardUi {
    val full = state.installsInApp
    return when (state.phase) {
        UpdatePhase.Downloading ->
            UpdateCardUi(
                title = R.string.updates_downloading_title,
                secondary = UpdateAction.Cancel,
                showNotes = true,
                showProgress = true,
            )
        UpdatePhase.Ready ->
            UpdateCardUi(
                title = R.string.updates_ready_title,
                body = R.string.updates_ready_body,
                primary = if (full) UpdateAction.Install else UpdateAction.OpenStore,
                showNotes = true,
            )
        UpdatePhase.NeedsPermission ->
            UpdateCardUi(
                title = R.string.updates_permission_title,
                body = R.string.updates_permission_body,
                primary = UpdateAction.OpenSettings,
                showNotes = true,
            )
        UpdatePhase.Installing ->
            UpdateCardUi(
                title = R.string.updates_installing_title,
                body = R.string.updates_installing_body,
                showNotes = true,
                showSpinner = true,
            )
        UpdatePhase.Installed ->
            UpdateCardUi(
                title = R.string.updates_installed_title,
                body = R.string.updates_installed_body,
                success = true,
            )
        else -> failedCard(state)
    }
}

private fun availableCard(full: Boolean) =
    UpdateCardUi(
        title = R.string.updates_available_title,
        primary = if (full) UpdateAction.Download else UpdateAction.OpenStore,
        showNotes = true,
        showSize = full,
    )

private fun failedCard(state: UpdatesUiState) =
    UpdateCardUi(
        title = R.string.updates_failed_title,
        problem = state.problem?.let { problemText(it) },
        primary = failedPrimary(state),
        showNotes = true,
        failed = true,
    )

/** Install when the APK is verified, else a fresh download or the store, else a new check. */
private fun failedPrimary(state: UpdatesUiState): UpdateAction =
    when {
        state.installsInApp && state.apkPath != null -> UpdateAction.Install
        state.release != null ->
            if (state.installsInApp) UpdateAction.Download else UpdateAction.OpenStore
        else -> UpdateAction.Check
    }

/** The sentence that explains a failed step. Each problem has its own sentence. */
@StringRes
internal fun problemText(problem: UpdateProblem): Int =
    when (problem) {
        UpdateProblem.Network -> R.string.updates_problem_network
        UpdateProblem.NothingPublished -> R.string.updates_problem_nothing_published
        UpdateProblem.BadResponse -> R.string.updates_problem_bad_response
        UpdateProblem.ChecksumMismatch -> R.string.updates_problem_checksum
        UpdateProblem.TooLarge -> R.string.updates_problem_too_large
        UpdateProblem.Storage -> R.string.updates_problem_storage
        UpdateProblem.InstallDeclined -> R.string.updates_problem_install_declined
        UpdateProblem.InstallBlocked -> R.string.updates_problem_install_blocked
        UpdateProblem.InstallConflict -> R.string.updates_problem_install_conflict
        UpdateProblem.InstallIncompatible -> R.string.updates_problem_install_incompatible
        UpdateProblem.InstallInvalid -> R.string.updates_problem_install_invalid
        UpdateProblem.InstallStorage -> R.string.updates_problem_install_storage
        UpdateProblem.InstallFailed -> R.string.updates_problem_install_failed
    }
