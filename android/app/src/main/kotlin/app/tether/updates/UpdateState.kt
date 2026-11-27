package app.tether.updates

import androidx.compose.runtime.Immutable

/** Where the update flow is. The screens map each phase to its own wording. */
enum class UpdatePhase {
    /** Nothing checked yet, or the last silent check failed. */
    Idle,

    /** A check is running. */
    Checking,

    /** The installed version is the newest one. */
    UpToDate,

    /** A newer release exists and has not been downloaded. */
    Available,

    /** The APK is downloading, then verified against the release checksums. */
    Downloading,

    /** The APK is downloaded and verified. The next step is Install. */
    Ready,

    /** Android must first allow this app to install packages. */
    NeedsPermission,

    /** The system installer has the APK. */
    Installing,

    /** The new version is in place. The process usually ends right after. */
    Installed,

    /** A step failed. [UpdatesUiState.problem] says which. */
    Failed,
}

/** Why a step failed. The screen owns the wording. */
enum class UpdateProblem {
    Network,
    NothingPublished,
    BadResponse,
    ChecksumMismatch,
    TooLarge,
    Storage,
    InstallDeclined,
    InstallBlocked,
    InstallConflict,
    InstallIncompatible,
    InstallInvalid,
    InstallStorage,
    InstallFailed,
    DownloadMissing,
}

/** A newer release, as the screens show it. */
@Immutable
data class ReleaseSummary(
    val version: String,
    val notes: String,
    val publishedAt: String,
    val apkSizeBytes: Long,
)

/** State of the update flow. */
@Immutable
data class UpdatesUiState(
    val currentVersion: String,
    val installsInApp: Boolean,
    val phase: UpdatePhase = UpdatePhase.Idle,
    val release: ReleaseSummary? = null,
    val knownNewerVersion: String? = null,
    val downloadedBytes: Long = 0L,
    val totalBytes: Long = 0L,
    val apkPath: String? = null,
    val problem: UpdateProblem? = null,
    /** True in a debuggable build. The flow never checks, so no update is ever offered. */
    val developmentBuild: Boolean = false,
) {
    /** True when a newer version is known, even before its notes are loaded. */
    val updateAvailable: Boolean
        get() = !developmentBuild && (release != null || knownNewerVersion != null)

    /** The newer version to name in a banner, or null. */
    val newerVersion: String?
        get() = if (developmentBuild) null else release?.version ?: knownNewerVersion

    /** Download progress from 0 to 100, or null while the size is not known. */
    val percent: Int?
        get() =
            if (totalBytes >
                0L
            ) {
                ((downloadedBytes * PERCENT) / totalBytes).toInt().coerceIn(0, PERCENT)
            } else {
                null
            }

    private companion object {
        const val PERCENT = 100
    }
}
