package app.tether.settings

import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import androidx.annotation.StringRes
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import app.tether.R

/**
 * One notification channel the app creates. The importance decides how each kind of notification
 * reaches the user, and the description says so in the same words.
 */
data class ChannelSpec(
    val id: String,
    @StringRes val nameRes: Int,
    @StringRes val descriptionRes: Int,
    val importance: Int,
)

/** The four channels the notification settings list, in the order they are shown. */
val TETHER_CHANNELS: List<ChannelSpec> =
    listOf(
        ChannelSpec(
            id = "approvals",
            nameRes = R.string.settings_area_channel_approvals,
            descriptionRes = R.string.settings_area_channel_approvals_desc,
            importance = NotificationManager.IMPORTANCE_HIGH,
        ),
        ChannelSpec(
            id = "run_status",
            nameRes = R.string.settings_area_channel_run_status,
            descriptionRes = R.string.settings_area_channel_run_status_desc,
            importance = NotificationManager.IMPORTANCE_LOW,
        ),
        ChannelSpec(
            id = "finished_turns",
            nameRes = R.string.settings_area_channel_finished,
            descriptionRes = R.string.settings_area_channel_finished_desc,
            importance = NotificationManager.IMPORTANCE_DEFAULT,
        ),
        ChannelSpec(
            id = "budget_errors",
            nameRes = R.string.settings_area_channel_budget,
            descriptionRes = R.string.settings_area_channel_budget_desc,
            importance = NotificationManager.IMPORTANCE_HIGH,
        ),
    )

/**
 * Creates every channel in [TETHER_CHANNELS]. Creating one that exists keeps the user's choices.
 */
fun createTetherChannels(context: Context) {
    val manager = context.getSystemService(NotificationManager::class.java) ?: return
    TETHER_CHANNELS.forEach { spec ->
        val channel = NotificationChannel(spec.id, context.getString(spec.nameRes), spec.importance)
        channel.description = context.getString(spec.descriptionRes)
        manager.createNotificationChannel(channel)
    }
}

/**
 * What the notification settings need from the system. Reads are cheap and safe on the main thread.
 */
interface NotificationSource {
    /** Whether the system lets this app post notifications at all. */
    fun enabled(): Boolean

    /** The importance of channel [channelId], or null when the channel does not exist. */
    fun importance(channelId: String): Int?

    /** True on Android 13 and later when the notification permission is not granted yet. */
    fun canAskForPermission(): Boolean
}

/** The real source over the platform notification manager. */
class SystemNotificationSource(
    private val context: Context,
) : NotificationSource {
    override fun enabled(): Boolean =
        NotificationManagerCompat.from(context).areNotificationsEnabled()

    override fun importance(channelId: String): Int? =
        context
            .getSystemService(NotificationManager::class.java)
            ?.getNotificationChannel(channelId)
            ?.importance

    override fun canAskForPermission(): Boolean =
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
}

/** The runtime permission that lets the app post notifications. It exists from Android 13 on. */
const val POST_NOTIFICATIONS = "android.permission.POST_NOTIFICATIONS"
