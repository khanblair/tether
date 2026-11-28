package app.tether.settings.executors

import androidx.compose.runtime.Immutable
import app.tether.linux.LinuxBackgroundMode

/** One sandbox pack row's state. Each state allows one action or none. */
sealed interface PackState {
    /** Listed but not installed. Install is offered. */
    data object NotInstalled : PackState

    /** A download is running. [total] is 0 until the core reports the size. */
    data class Downloading(
        val downloaded: Long,
        val total: Long,
    ) : PackState

    /** The download is complete and the core is checking it. */
    data object Verifying : PackState

    /** Installed and checked. */
    data class Ready(
        val version: String,
    ) : PackState

    /** The last install did not finish. Try again is offered. */
    data class Failed(
        val failure: ExecutorsFailure,
    ) : PackState

    /** This build has no manifest or key for the pack, so nothing can be installed. */
    data object NotAvailable : PackState

    /** The screen stopped waiting for a download. The core's run may still be going. */
    data object Detached : PackState
}

/** One pack row: what the core listed, plus the state the screen shows for it. */
data class PackRow(
    val id: String,
    val version: String,
    val sizeBytes: Long,
    val state: PackState,
)

/** Why Linux mode cannot be set up in this build. */
enum class LinuxUnavailable {
    /** The Play build has no Linux mode at all. */
    ThisBuild,

    /** The build has Linux mode but not its manifest or key. */
    NotConfigured,
}

/** The Linux mode card's state. */
sealed interface LinuxState {
    /** The readiness check has not returned yet. */
    data object Loading : LinuxState

    /** Linux mode cannot be set up in this build. Nothing is offered. */
    data class Unavailable(
        val reason: LinuxUnavailable,
    ) : LinuxState

    /** Not set up yet. Install is offered. */
    data object NotInstalled : LinuxState

    /** A setup is running. Both counts are 0 before the first report. */
    data class SettingUp(
        val downloaded: Long,
        val total: Long,
    ) : LinuxState

    /** Set up and ready. Repair is offered, Remove only when the core supports it. */
    data object Ready : LinuxState

    /** The last setup or the readiness check failed. Repair is offered. */
    data class Broken(
        val failure: ExecutorsFailure,
        val detail: String? = null,
    ) : LinuxState
}

/** The network switch's state for one project. */
sealed interface NetworkState {
    /** No project is open, so there is no switch. */
    data object NoProject : NetworkState

    /** The saved value is being read. */
    data object Loading : NetworkState

    /** The saved value. [saving] is true while a change is written and read back. */
    data class Loaded(
        val allowed: Boolean,
        val saving: Boolean = false,
        val failure: ExecutorsFailure? = null,
    ) : NetworkState

    /** The saved value could not be read. */
    data class LoadFailed(
        val failure: ExecutorsFailure,
    ) : NetworkState
}

/** The CI section's state for one project. */
sealed interface CiState {
    /** No project is open, so CI has nothing to check. */
    data object NoProject : CiState

    /** The readiness check has not returned yet. */
    data object Loading : CiState

    /** The readiness check result. */
    data class Status(
        val readiness: CiReadiness,
    ) : CiState

    /** The readiness check failed. */
    data class Failed(
        val failure: ExecutorsFailure,
    ) : CiState
}

/**
 * Everything the executor screen shows. [packCore] is the core's list. [packOverrides] holds the
 * states of installs in progress or failed, which the core's list does not know about.
 */
@Immutable
data class ExecutorsUiState(
    val packsLoaded: Boolean = false,
    val packCore: List<PackSummary> = emptyList(),
    val packOverrides: Map<String, PackState> = emptyMap(),
    val packsFailure: ExecutorsFailure? = null,
    val packInstallAvailable: Boolean = false,
    /** The host the packs download from, shown so the user sees where it comes from. */
    val packSource: String? = null,
    val linux: LinuxState = LinuxState.Loading,
    val linuxSetupInBuild: Boolean = false,
    val linuxBackgroundMode: LinuxBackgroundMode = LinuxBackgroundMode.WhileRunning,
    /** The host the Linux root filesystem downloads from. */
    val linuxSource: String? = null,
    val network: NetworkState = NetworkState.Loading,
    val ci: CiState = CiState.Loading,
)

/** The pack rows to show: the core's list, with any install in progress or failed on top. */
fun ExecutorsUiState.packRows(): List<PackRow> =
    packCore.map { pack ->
        PackRow(
            id = pack.id,
            version = pack.version,
            sizeBytes = pack.sizeBytes,
            state = packStateOf(pack),
        )
    }

private fun ExecutorsUiState.packStateOf(pack: PackSummary): PackState {
    packOverrides[pack.id]?.let { return it }
    return when {
        pack.installed -> PackState.Ready(pack.version)
        !packInstallAvailable -> PackState.NotAvailable
        else -> PackState.NotInstalled
    }
}
