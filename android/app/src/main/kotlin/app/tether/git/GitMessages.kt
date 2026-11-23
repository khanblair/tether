package app.tether.git

import androidx.annotation.StringRes
import app.tether.R

/** The message for a failure. Screens show it; no core text ever reaches the UI. */
@StringRes
internal fun GitFailure.messageRes(): Int =
    when (this) {
        GitFailure.NotRepository -> R.string.git_failure_not_repository
        GitFailure.NotFound -> R.string.git_failure_not_found
        GitFailure.InvalidInput -> R.string.git_failure_invalid_input
        GitFailure.Unavailable -> R.string.git_failure_unavailable
        GitFailure.RepositoryFailed -> R.string.git_failure_repository
    }

/** The tab label. */
@StringRes
internal fun GitTab.labelRes(): Int =
    when (this) {
        GitTab.Changes -> R.string.git_tab_changes
        GitTab.Branches -> R.string.git_tab_branches
        GitTab.History -> R.string.git_tab_history
    }

/** The name of a change, read by TalkBack and shown in the mark's description. */
@StringRes
internal fun ChangeKind.labelRes(): Int =
    when (this) {
        ChangeKind.Added -> R.string.git_state_added
        ChangeKind.Modified -> R.string.git_state_modified
        ChangeKind.Deleted -> R.string.git_state_deleted
        ChangeKind.Renamed -> R.string.git_state_renamed
        ChangeKind.Untracked -> R.string.git_state_untracked
        ChangeKind.Conflicted -> R.string.git_state_conflicted
        ChangeKind.Changed -> R.string.git_state_changed
    }

/** The one-letter mark for a change. */
@StringRes
internal fun ChangeKind.markRes(): Int =
    when (this) {
        ChangeKind.Added -> R.string.git_mark_added
        ChangeKind.Modified -> R.string.git_mark_modified
        ChangeKind.Deleted -> R.string.git_mark_deleted
        ChangeKind.Renamed -> R.string.git_mark_renamed
        ChangeKind.Untracked -> R.string.git_mark_untracked
        ChangeKind.Conflicted -> R.string.git_mark_conflicted
        ChangeKind.Changed -> R.string.git_mark_changed
    }
