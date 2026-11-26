package app.tether.crash

import android.content.Context
import android.content.pm.PackageManager
import java.time.Instant

/** The folder under `filesDir` that holds the crash report. */
internal const val CRASH_DIR = "crash"

/**
 * Saves a report for every uncaught exception, then passes the crash on to the previous handler,
 * so the system still closes the app. A failed save never hides the crash.
 */
object CrashRecorder {
    /** Installs the handler. Call it once, first in `Application.onCreate`. */
    fun install(context: Context) {
        val store = FileCrashStore(context.filesDir.resolve(CRASH_DIR))
        val version = versionName(context)
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            try {
                store.write(formatCrashReport(Instant.now(), version, thread.name, error))
            } catch (
                @Suppress("TooGenericExceptionCaught") _: Throwable,
            ) {
                // Nothing else can be done here. The previous handler still runs below.
            } finally {
                previous?.uncaughtException(thread, error)
            }
        }
    }

    private fun versionName(context: Context): String =
        try {
            context.packageManager
                .getPackageInfo(context.packageName, 0)
                .versionName
                .orEmpty()
        } catch (_: PackageManager.NameNotFoundException) {
            UNKNOWN_VERSION
        }

    private const val UNKNOWN_VERSION = "unknown"
}
