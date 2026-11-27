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
            val text = intent?.getStringExtra(EXTRA_TEXT) ?: return START_NOT_STICKY
            startForeground(NOTIFICATION_ID, notification(text))
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun notification(text: String) =
        NotificationCompat
            .Builder(this, LINUX_CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_launcher_monochrome)
            .setContentTitle(getString(R.string.linux_background_notification_title))
            .setContentText(text)
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
        const val EXTRA_TEXT = "notification_text"
        const val LINUX_CHANNEL_ID = "linux_session"
        private const val NOTIFICATION_ID = 7101
        private const val NOTIFICATION_REQUEST_CODE = 7102
    }
}
