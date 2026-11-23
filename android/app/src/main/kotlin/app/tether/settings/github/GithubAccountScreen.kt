package app.tether.settings.github

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.settings.SettingsPage

/**
 * The GitHub account screen. Signed out: the device flow (or one sentence and the token path when
 * the build has no client ID), then the token alternative. Signed in: the account card with Sign
 * out. Leaving the screen stops the device poll. [onOpenIdentity] opens the Git identity form.
 * [onOpenUrl] receives the exact URL to open.
 */
@Composable
fun GithubAccountScreen(
    state: AccountUiState,
    onBack: () -> Unit,
    onSignInWithDevice: () -> Unit,
    onStopPolling: () -> Unit,
    onOpenUrl: (String) -> Unit,
    onTokenChange: (String) -> Unit,
    onSignInWithToken: () -> Unit,
    onSignOut: () -> Unit,
    onOpenIdentity: () -> Unit,
) {
    DisposableEffect(Unit) {
        onDispose { onStopPolling() }
    }
    SettingsPage(
        title = stringResource(R.string.github_account_title),
        onBack = onBack,
    ) {
        if (state.loading) return@SettingsPage
        GitHubAccountCard(status = state.status, onSignOut = onSignOut)
        TetherTextButton(
            label = stringResource(R.string.github_account_edit_identity),
            onClick = onOpenIdentity,
        )
        if (!state.status.signedIn) {
            DeviceSection(
                state = state,
                onSignInWithDevice = onSignInWithDevice,
                onOpenUrl = onOpenUrl,
            )
            GithubTokenForm(
                token = state.token,
                emptyToken = state.emptyToken,
                busy = state.busy,
                onTokenChange = onTokenChange,
                onSignIn = onSignInWithToken,
            )
        }
        state.failure?.let { failure ->
            InlineBanner(
                text = stringResource(failure.messageRes()),
                tone = BannerTone.Danger,
            )
        }
    }
}

/** The device-flow part of the signed-out screen. */
@Composable
private fun DeviceSection(
    state: AccountUiState,
    onSignInWithDevice: () -> Unit,
    onOpenUrl: (String) -> Unit,
) {
    when (val device = state.device) {
        DeviceStep.Idle ->
            if (state.deviceAvailable) {
                PrimaryButton(
                    label = stringResource(R.string.github_account_sign_in),
                    onClick = onSignInWithDevice,
                )
            } else {
                Text(
                    text = stringResource(R.string.github_account_device_unavailable),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text2,
                )
            }

        DeviceStep.Starting ->
            Text(
                text = stringResource(R.string.github_device_starting),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )

        is DeviceStep.Waiting ->
            DeviceCodePanel(
                grant = device.grant,
                expiresAtMillis = device.expiresAtMillis,
                onOpen = { onOpenUrl(device.grant.verificationUri) },
            )

        DeviceStep.Denied -> EndedDevice(R.string.github_device_denied, onSignInWithDevice)

        DeviceStep.Expired -> EndedDevice(R.string.github_device_expired, onSignInWithDevice)
    }
}

/** A device flow that ended without approval: the reason and a new code. */
@Composable
private fun EndedDevice(
    reason: Int,
    onNewCode: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space3)) {
        Text(
            text = stringResource(reason),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        PrimaryButton(
            label = stringResource(R.string.github_device_new_code),
            onClick = onNewCode,
        )
    }
}
