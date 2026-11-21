package app.tether.settings

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.components.BadgeTone
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.StatusBadge
import app.tether.design.components.TetherTextButton

/**
 * Notification channels and the permission state. When the system blocks notifications, the
 * screen says what that costs and offers the system settings. Android 13 and later ask for the
 * permission only from the button here.
 */
@Composable
fun NotificationsScreen(
    viewModel: NotificationsViewModel,
    onBack: () -> Unit,
    onOpenChannel: (String) -> Unit,
    onOpenAppSettings: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { viewModel.refresh() }
    val askPermission =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
            viewModel.refresh()
        }
    SettingsPage(
        title = stringResource(R.string.settings_area_notifications_title),
        onBack = onBack,
    ) {
        if (state.enabled) {
            InlineBanner(
                text = stringResource(R.string.settings_area_notifications_on),
                tone = BannerTone.Success,
            )
        } else {
            BlockedNotes(
                canAskForPermission = state.canAskForPermission,
                onAskForPermission = {
                    askPermission.launch(
                        POST_NOTIFICATIONS,
                    )
                },
                onOpenAppSettings = onOpenAppSettings,
            )
        }
        SectionTitle(text = stringResource(R.string.settings_area_channels))
        ChannelRows(state.channels, onOpenChannel)
    }
}

/** What the user can do while the system blocks notifications for this app. */
@Composable
private fun BlockedNotes(
    canAskForPermission: Boolean,
    onAskForPermission: () -> Unit,
    onOpenAppSettings: () -> Unit,
) {
    InlineBanner(
        title = stringResource(R.string.settings_area_notifications_off_title),
        text = stringResource(R.string.settings_area_notifications_off_body),
        tone = BannerTone.Danger,
    )
    if (canAskForPermission) {
        PrimaryButton(
            label = stringResource(R.string.settings_area_allow_notifications),
            onClick = onAskForPermission,
        )
    }
    TetherTextButton(
        label = stringResource(R.string.settings_area_open_system_settings),
        onClick = onOpenAppSettings,
    )
}

/** One row per channel, with its on or off state as a label, so color is not the only signal. */
@Composable
private fun ChannelRows(
    channels: List<ChannelRow>,
    onOpenChannel: (String) -> Unit,
) {
    ListGroup {
        channels.forEach { channel ->
            ListRow(
                title = stringResource(channel.nameRes),
                subtitle = stringResource(channel.descriptionRes),
                trailing = { ChannelState(channel.on) },
                onClick = { onOpenChannel(channel.id) },
            )
        }
    }
}

@Composable
private fun ChannelState(on: Boolean) {
    val label = if (on) R.string.settings_area_channel_on else R.string.settings_area_channel_off
    StatusBadge(
        label = stringResource(label),
        tone = if (on) BadgeTone.Success else BadgeTone.Neutral,
    )
}
