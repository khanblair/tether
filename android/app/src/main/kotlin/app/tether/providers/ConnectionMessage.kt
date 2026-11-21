package app.tether.providers

import androidx.annotation.StringRes
import app.tether.R
import app.tether.core.generated.ConnectionStatusView
import app.tether.design.components.BannerTone

/** The title, body, and tone a failed connection result shows. */
data class ConnectionMessage(
    @StringRes val title: Int,
    @StringRes val body: Int,
    val tone: BannerTone,
)

/**
 * The message for a connection result that did not pass, or null for a result that needs none:
 * an accepted key, or a status that has not been tested yet.
 */
fun connectionMessage(status: ConnectionStatusView): ConnectionMessage? =
    when (status) {
        ConnectionStatusView.BAD_KEY ->
            ConnectionMessage(
                R.string.provider_detail_rejected_title,
                R.string.provider_detail_rejected_body,
                BannerTone.Danger,
            )
        ConnectionStatusView.NETWORK_ERROR ->
            ConnectionMessage(
                R.string.provider_detail_offline_title,
                R.string.provider_detail_offline_body,
                BannerTone.Warning,
            )
        ConnectionStatusView.BLOCKED ->
            ConnectionMessage(
                R.string.provider_detail_blocked_title,
                R.string.provider_detail_blocked_body,
                BannerTone.Danger,
            )
        ConnectionStatusView.OK,
        ConnectionStatusView.UNSET,
        ConnectionStatusView.TESTING,
        -> null
    }
