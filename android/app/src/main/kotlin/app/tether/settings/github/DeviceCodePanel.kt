package app.tether.settings.github

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace
import kotlinx.coroutines.delay

/** How often the countdown redraws, in milliseconds. */
private const val COUNTDOWN_TICK_MS = 1_000L

/** Seconds in one minute, for the countdown text. */
private const val SECONDS_PER_MINUTE = 60L

/**
 * The device-flow code: the one-time code large in the mono style, Copy and Open GitHub, the
 * exact URL that Open GitHub goes to, a countdown to expiry, and a waiting line. [onOpen] is the
 * only thing that leaves the app, so it is the only place the URL is used.
 */
@Composable
fun DeviceCodePanel(
    grant: DeviceGrant,
    expiresAtMillis: Long,
    onOpen: () -> Unit,
    modifier: Modifier = Modifier,
) {
    var copied by remember(grant.userCode) { mutableStateOf(false) }
    val context = LocalContext.current
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = stringResource(R.string.github_device_title),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
        Text(
            text = stringResource(R.string.github_device_code_label),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
        Text(
            text = grant.userCode,
            style =
                MaterialTheme.typography.displaySmall.copy(
                    fontFamily = TetherTheme.mono.fontFamily,
                ),
            color = TetherTheme.colors.text,
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(
                        TetherTheme.colors.surfaceSunken,
                        RoundedCornerShape(TetherRadius.md),
                    ).padding(TetherSpace.space4),
        )
        CountdownLine(expiresAtMillis)
        TetherTextButton(
            label =
                stringResource(
                    if (copied) R.string.github_device_copied else R.string.github_device_copy,
                ),
            onClick = {
                copyToClipboard(context, grant.userCode)
                copied = true
            },
        )
        PrimaryButton(
            label = stringResource(R.string.github_device_open),
            onClick = onOpen,
        )
        Text(
            text = stringResource(R.string.github_device_open_target, grant.verificationUri),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
        Row(verticalAlignment = Alignment.CenterVertically) {
            TetherIcon(
                icon = TetherIcons.Loader,
                contentDescription = null,
                tint = TetherTheme.colors.text2,
            )
            Text(
                text = stringResource(R.string.github_device_waiting),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text2,
                modifier = Modifier.padding(start = TetherSpace.space2),
            )
        }
    }
}

/** The expiry countdown. It redraws each second; the text uses minutes until one minute is left. */
@Composable
private fun CountdownLine(expiresAtMillis: Long) {
    val secondsLeft by produceState(
        initialValue = secondsUntil(expiresAtMillis, System.currentTimeMillis()),
        key1 = expiresAtMillis,
    ) {
        while (true) {
            value = secondsUntil(expiresAtMillis, System.currentTimeMillis())
            delay(COUNTDOWN_TICK_MS)
        }
    }
    val text =
        if (secondsLeft >= SECONDS_PER_MINUTE) {
            val minutes = ((secondsLeft + SECONDS_PER_MINUTE - 1) / SECONDS_PER_MINUTE).toInt()
            pluralStringResource(R.plurals.github_device_expires_minutes, minutes, minutes)
        } else {
            val seconds = secondsLeft.toInt()
            pluralStringResource(R.plurals.github_device_expires_seconds, seconds, seconds)
        }
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
    )
}
