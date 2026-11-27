package app.tether.linux

import android.content.Context

/** The Play build has no Linux mode or foreground service. */
private object PlayLinuxServiceController : LinuxServiceController {
    override fun apply(action: ServiceAction): Boolean = true
}

/** Creates the Play build's no-op service bridge. */
@Suppress("UNUSED_PARAMETER")
fun createLinuxServiceController(context: Context): LinuxServiceController =
    PlayLinuxServiceController
