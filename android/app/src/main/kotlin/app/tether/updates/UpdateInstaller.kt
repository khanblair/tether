package app.tether.updates

import java.io.File

/** What an update installer can do on this build. */
interface UpdateInstaller {
    /** True on the full build, which installs updates itself. False on the Play build. */
    val installsInApp: Boolean

    /** True when the app may install packages now. Always false when [installsInApp] is false. */
    fun canRequestInstalls(): Boolean

    /** Opens the system screen where the user allows this app to install packages. */
    fun openInstallPermissionSettings()

    /**
     * Hands the verified APK in [apk] to the system installer. Blocks while it copies the file,
     * so call it off the main thread. Results arrive on [listener], possibly on the main thread.
     */
    fun install(
        apk: File,
        listener: InstallListener,
    )

    /** Opens this app's Google Play page. Does nothing on the full build. */
    fun openStorePage()
}

/** Progress of one install, reported by [UpdateInstaller.install]. */
interface InstallListener {
    /** The system installer shows its confirmation screen. */
    fun onConfirmationShown()

    /**
     * The install finished and the new version is in place. The process usually ends right after.
     */
    fun onSuccess()

    /** The install failed or the user declined. */
    fun onFailure(failure: InstallFailure)
}

/** Why an install did not finish. */
enum class InstallFailure {
    Cancelled,
    Blocked,
    Conflict,
    Incompatible,
    Invalid,
    Storage,
    Other,
}
