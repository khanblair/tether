package app.tether.projectarchive

import app.tether.core.generated.ProjectRuntimeException

/**
 * Why an import or export did not finish. Each value has one sentence in
 * `strings_projectarchive.xml`, given by [messageRes].
 */
enum class ArchiveReason {
    /** The core rejected the project name. */
    InvalidName,

    /** The ZIP holds a file path that would leave the project folder. */
    InvalidEntry,

    /** The ZIP has more entries than the core allows. */
    TooManyEntries,

    /** The ZIP unpacks to more bytes than the core allows. */
    TooLarge,

    /** The ZIP compresses so much that the core refuses it. */
    UnsafeRatio,

    /** The file is not a ZIP archive. */
    NotZip,

    /** The core could not write to the destination path. */
    InvalidDestination,

    /** Storage the archive code needs is unavailable. */
    StorageUnavailable,

    /** The operation was cancelled. */
    Cancelled,

    /** The project no longer exists. */
    ProjectNotFound,

    /** The project database or workspace folder could not be used. */
    Unavailable,

    /** The selected workspace folder failed validation. */
    InvalidWorkspaceFolder,

    /** The core reported an error that cannot come from an archive call. */
    Unexpected,

    /** The app could not read the file the person picked. */
    ReadFailed,

    /** The app could not save the ZIP to the place the person chose. */
    WriteFailed,

    /** The core reported an archive detail this build does not map to a sentence. */
    UnknownDetail,
}

/** The result of an import or export call: the value, or the reason it failed. */
sealed interface ArchiveResult<out T> {
    /** The call finished. */
    data class Succeeded<out T>(
        val value: T,
    ) : ArchiveResult<T>

    /** The call did not finish. Nothing new was kept. */
    data class Failed(
        val reason: ArchiveReason,
    ) : ArchiveResult<Nothing>
}

/**
 * Maps the core's archive detail text to a reason. The detail strings come from
 * `core/crates/tether-ffi/src/archive.rs`. An unknown detail maps to [ArchiveReason.UnknownDetail].
 */
fun archiveReasonForDetail(detail: String): ArchiveReason =
    when (detail) {
        "invalid archive entry" -> ArchiveReason.InvalidEntry
        "archive has too many entries" -> ArchiveReason.TooManyEntries
        "archive is too large" -> ArchiveReason.TooLarge
        "archive compression ratio is unsafe" -> ArchiveReason.UnsafeRatio
        "archive operation cancelled" -> ArchiveReason.Cancelled
        "archive destination is invalid" -> ArchiveReason.InvalidDestination
        "file is not a ZIP archive" -> ArchiveReason.NotZip
        "archive storage is unavailable" -> ArchiveReason.StorageUnavailable
        else -> ArchiveReason.UnknownDetail
    }

/** Maps every core error variant to the reason shown to the person. */
fun ProjectRuntimeException.toArchiveReason(): ArchiveReason =
    if (isWorkspaceFolderProblem()) {
        ArchiveReason.InvalidWorkspaceFolder
    } else {
        when (this) {
            is ProjectRuntimeException.InvalidName -> ArchiveReason.InvalidName
            is ProjectRuntimeException.InvalidDefaults -> ArchiveReason.Unexpected
            is ProjectRuntimeException.BypassNeedsConfirmation -> ArchiveReason.Unexpected
            is ProjectRuntimeException.Archive -> archiveReasonForDetail(detail)
            is ProjectRuntimeException.Cancelled -> ArchiveReason.Cancelled
            is ProjectRuntimeException.NotFound -> ArchiveReason.ProjectNotFound
            is ProjectRuntimeException.Unavailable -> ArchiveReason.Unavailable
            else -> ArchiveReason.Unexpected
        }
    }

private fun ProjectRuntimeException.isWorkspaceFolderProblem(): Boolean =
    when (this) {
        is ProjectRuntimeException.RootNotAbsolute,
        is ProjectRuntimeException.RootCannotBeCreated,
        is ProjectRuntimeException.RootNotDirectory,
        is ProjectRuntimeException.RootIsSymlink,
        is ProjectRuntimeException.RootNotWritable,
        is ProjectRuntimeException.RootInsidePrivateRoot,
        is ProjectRuntimeException.RootIsFilesystemRoot,
        is ProjectRuntimeException.RootInsideProject,
        is ProjectRuntimeException.RootUnavailable,
        -> true
        else -> false
    }
