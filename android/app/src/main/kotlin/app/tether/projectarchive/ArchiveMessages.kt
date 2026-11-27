package app.tether.projectarchive

import androidx.annotation.StringRes
import app.tether.R

/** The sentence for each reason, from `strings_projectarchive.xml`. */
@StringRes
@Suppress("CyclomaticComplexMethod")
fun ArchiveReason.messageRes(): Int =
    when (this) {
        ArchiveReason.InvalidName -> R.string.archive_reason_invalid_name
        ArchiveReason.InvalidEntry -> R.string.archive_reason_invalid_entry
        ArchiveReason.TooManyEntries -> R.string.archive_reason_too_many_entries
        ArchiveReason.TooLarge -> R.string.archive_reason_too_large
        ArchiveReason.UnsafeRatio -> R.string.archive_reason_unsafe_ratio
        ArchiveReason.NotZip -> R.string.archive_reason_not_zip
        ArchiveReason.InvalidDestination -> R.string.archive_reason_invalid_destination
        ArchiveReason.StorageUnavailable -> R.string.archive_reason_storage
        ArchiveReason.Cancelled -> R.string.archive_reason_cancelled
        ArchiveReason.ProjectNotFound -> R.string.archive_reason_project_not_found
        ArchiveReason.Unavailable -> R.string.archive_reason_unavailable
        ArchiveReason.InvalidWorkspaceFolder -> R.string.archive_reason_invalid_workspace_folder
        ArchiveReason.Unexpected -> R.string.archive_reason_unexpected
        ArchiveReason.ReadFailed -> R.string.archive_reason_read
        ArchiveReason.WriteFailed -> R.string.archive_reason_write
        ArchiveReason.UnknownDetail -> R.string.archive_reason_unknown
    }
