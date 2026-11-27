package app.tether.linux

import android.content.Context
import app.tether.core.generated.ExecutorPathsView
import app.tether.settings.executors.ExecutorBuildConfig
import java.io.File

/** Distribution-specific Linux mode setup and build configuration. */
interface LinuxSetup {
    /** Returns the executor settings exposed by this flavor. */
    fun executorBuildConfig(context: Context): ExecutorBuildConfig

    /** Configures the native Linux executors at process start. */
    fun configureAtStart(
        context: Context,
        configureExecutors: (ExecutorPathsView) -> Unit,
    )
}

/** Converts an optional resource value to the null form expected by the core. */
internal fun resourceValueOrNull(value: String): String? = value.trim().takeIf { it.isNotEmpty() }

/** Builds the paths passed to the core from paths that are valid for this app process. */
internal fun executorPaths(
    nativeLibraryDir: String,
    filesDir: String,
    packVerifyingKeyHex: String?,
    rootfsVerifyingKeyHex: String?,
): ExecutorPathsView {
    val nativeDir = File(nativeLibraryDir)
    val files = File(filesDir)
    return ExecutorPathsView(
        packsRoot = File(files, RUNTIMES_DIR).path,
        linuxRoot = File(files, LINUX_DIR).path,
        prootPath = File(nativeDir, PROOT_LIBRARY).path,
        prootLoaderPath = File(nativeDir, PROOT_LOADER).path,
        packVerifyingKeyHex = packVerifyingKeyHex,
        rootfsVerifyingKeyHex = rootfsVerifyingKeyHex,
    )
}

/** Configures Linux only when both native PRoot files are available to this process. */
internal fun configureLinuxIfAvailable(
    paths: ExecutorPathsView,
    fileExists: (String) -> Boolean,
    configureExecutors: (ExecutorPathsView) -> Unit,
): Boolean {
    val available =
        paths.prootPath?.let(fileExists) == true &&
            paths.prootLoaderPath?.let(fileExists) == true
    if (available) configureExecutors(paths)
    return available
}

private const val PROOT_LIBRARY = "libproot.so"
private const val PROOT_LOADER = "libproot-loader.so"
private const val RUNTIMES_DIR = "runtimes"
private const val LINUX_DIR = "linux"
