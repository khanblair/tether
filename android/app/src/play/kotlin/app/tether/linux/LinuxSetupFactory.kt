package app.tether.linux

import android.content.Context
import app.tether.core.generated.ExecutorPathsView
import app.tether.settings.executors.ExecutorBuildConfig

/** Creates the no-op Linux mode capability for the Play build. */
fun createLinuxSetup(): LinuxSetup = PlayLinuxSetup

private object PlayLinuxSetup : LinuxSetup {
    override fun executorBuildConfig(context: Context): ExecutorBuildConfig =
        ExecutorBuildConfig(
            packManifestUrl = null,
            rootfsManifestUrl = null,
            linuxSupported = false,
        )

    override fun configureAtStart(
        context: Context,
        configureExecutors: (ExecutorPathsView) -> Unit,
    ) = Unit
}
