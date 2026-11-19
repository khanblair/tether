package app.tether.updates

import app.tether.core.generated.DownloadProgress
import app.tether.core.generated.UpdateCheckView
import app.tether.core.generated.Updater

/**
 * What the update screens need from the Rust core. Every call blocks, so callers run it off the
 * main thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface UpdateGateway {
    /** Asks GitHub for the latest release and compares it with [currentVersion]. */
    fun check(currentVersion: String): UpdateCheckView

    /**
     * Downloads the APK of the release that the last [check] found for [version], verifies it
     * against the published checksums, and returns its absolute path.
     */
    fun download(
        version: String,
        progress: DownloadProgress,
    ): String

    /** Stops a download in flight. Does nothing when none runs. */
    fun cancel()
}

/**
 * The real gateway: forwards each call to the UniFFI [Updater]. The object is created on the first
 * call, which runs off the main thread.
 */
class CoreUpdateGateway(
    lazyUpdater: Lazy<Updater>,
) : UpdateGateway {
    private val updater: Updater by lazyUpdater

    override fun check(currentVersion: String): UpdateCheckView = updater.check(currentVersion)

    override fun download(
        version: String,
        progress: DownloadProgress,
    ): String = updater.download(version, progress)

    override fun cancel() = updater.cancel()
}
