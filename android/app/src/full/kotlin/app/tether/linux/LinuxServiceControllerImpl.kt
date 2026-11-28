package app.tether.linux

import android.annotation.SuppressLint
import android.app.ForegroundServiceStartNotAllowedException
import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.core.content.ContextCompat

/** The full build's foreground-service bridge. */
@SuppressLint("NewApi")
class LinuxServiceControllerImpl(
    private val context: Context,
) : LinuxServiceController {
    override fun apply(action: ServiceAction): Boolean =
        when (action) {
            ServiceAction.Stop -> stop()
            is ServiceAction.Run -> start(action.notice)
        }

    private fun start(notice: LinuxNotice): Boolean {
        val intent =
            Intent(context, LinuxSessionService::class.java)
                .putExtra(LinuxSessionService.EXTRA_KIND, notice.kind.name)
                .putExtra(LinuxSessionService.EXTRA_SERVERS, notice.servers)
        try {
            ContextCompat.startForegroundService(context, intent)
        } catch (_: ForegroundServiceStartNotAllowedException) {
            Log.w(TAG, "Linux foreground service start not allowed")
            return false
        }
        return true
    }

    private fun stop(): Boolean {
        val intent =
            Intent(context, LinuxSessionService::class.java)
                .setAction(LinuxSessionService.ACTION_STOP)
        try {
            context.startService(intent)
        } catch (_: ForegroundServiceStartNotAllowedException) {
            context.stopService(intent)
        }
        return true
    }

    private companion object {
        const val TAG = "TetherLinux"
    }
}

/** Creates the full build's service bridge. */
fun createLinuxServiceController(context: Context): LinuxServiceController =
    LinuxServiceControllerImpl(context)
