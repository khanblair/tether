package app.tether.linux

import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.IBinder
import androidx.core.app.NotificationCompat
import app.tether.MainActivity
import app.tether.R

/** Keeps Linux child processes out of the cached app freezer in the full build. */
class LinuxSessionService : Service() {
    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf()
        } else {
            val notice =
                LinuxNotice.fromExtras(
                    kindName = intent?.getStringExtra(EXTRA_KIND),
                    servers = intent?.getIntExtra(EXTRA_SERVERS, 0) ?: 0,
                ) ?: return START_NOT_STICKY
            startForeground(NOTIFICATION_ID, notification(notice))
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun noticeText(notice: LinuxNotice): String =
        when (notice.kind) {
            LinuxNoticeKind.ShellOpen -> getString(R.string.linux_notification_shell_open)
            LinuxNoticeKind.Ready -> getString(R.string.linux_notification_ready)
            LinuxNoticeKind.Servers ->
                resources.getQuantityString(
                    R.plurals.linux_notification_servers,
                    notice.servers,
                    notice.servers,
                )
            LinuxNoticeKind.ShellAndServers ->
                resources.getQuantityString(
                    R.plurals.linux_notification_shell_and_servers,
                    notice.servers,
                    notice.servers,
                )
        }

    private fun notification(notice: LinuxNotice) =
        NotificationCompat
            .Builder(this, LINUX_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(getString(R.string.linux_background_notification_title))
            .setContentText(noticeText(notice))
            .setOngoing(true)
            .setContentIntent(contentIntent())
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()

    private fun contentIntent(): PendingIntent =
        PendingIntent.getActivity(
            this,
            NOTIFICATION_REQUEST_CODE,
            Intent(this, MainActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )

    companion object {
        const val ACTION_STOP = "app.tether.linux.STOP"
        const val EXTRA_KIND = "notification_kind"
        const val EXTRA_SERVERS = "notification_servers"
        const val LINUX_CHANNEL_ID = "linux_session"
        private const val NOTIFICATION_ID = 7101
        private const val NOTIFICATION_REQUEST_CODE = 7102
    }
}
