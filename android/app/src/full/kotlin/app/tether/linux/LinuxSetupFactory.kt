package app.tether.linux

import android.content.Context
import android.os.Build
import android.util.Log
import app.tether.R
import app.tether.core.generated.ExecutorPathsView
import app.tether.settings.executors.ExecutorBuildConfig
import java.io.File

/** Creates Linux mode for the full build. */
fun createLinuxSetup(): LinuxSetup = FullLinuxSetup

private object FullLinuxSetup : LinuxSetup {
    override fun executorBuildConfig(context: Context): ExecutorBuildConfig =
        ExecutorBuildConfig(
            packManifestUrl = null,
            rootfsManifestUrl =
                resourceValueOrNull(
                    context.getString(R.string.linux_rootfs_manifest_url),
                ),
            linuxSupported = Build.SUPPORTED_ABIS.contains("arm64-v8a"),
        )

    override fun configureAtStart(
        context: Context,
        configureExecutors: (ExecutorPathsView) -> Unit,
    ) {
        val nativeLibraryDir = context.applicationInfo.nativeLibraryDir
        val paths =
            executorPaths(
                nativeLibraryDir = nativeLibraryDir,
                filesDir = context.filesDir.path,
                packVerifyingKeyHex =
                    resourceValueOrNull(context.getString(R.string.runtime_pack_key_hex)),
                rootfsVerifyingKeyHex =
                    resourceValueOrNull(context.getString(R.string.linux_rootfs_key_hex)),
            )
        val configured =
            configureLinuxIfAvailable(
                paths = paths,
                fileExists = { path -> File(path).isFile },
                configureExecutors = configureExecutors,
            )
        if (!configured) Log.i(TAG, "Linux mode unavailable: PRoot library is missing")
    }
}

private const val TAG = "TetherLinux"
