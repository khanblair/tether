package app.tether.settings.executors

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherSpace

/**
 * Linux mode's status and its actions. [onRepair] and [onRemove] are null when the build cannot
 * offer them, and a null action is not drawn. [source] is the host the root filesystem downloads
 * from. Remove needs a confirmation before it is wired.
 */
@Composable
fun LinuxSetupCard(
    state: LinuxState,
    onInstall: () -> Unit,
    onRepair: (() -> Unit)?,
    modifier: Modifier = Modifier,
    source: String? = null,
    onRemove: (() -> Unit)? = null,
) {
    val colors = TetherTheme.colors
    val body = MaterialTheme.typography.bodySmall
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            linuxMark(state)?.let { StatusMark(state = it) }
            Text(
                text = linuxHeadline(state),
                style = MaterialTheme.typography.titleSmall,
                color = if (state is LinuxState.Broken) colors.danger else colors.text,
            )
        }
        when (state) {
            LinuxState.Loading -> Unit
            is LinuxState.Unavailable -> {
                Text(text = state.reason.text(), style = body, color = colors.text2)
            }
            LinuxState.NotInstalled -> {
                Text(
                    text = stringResource(R.string.executors_linux_not_installed_body),
                    style = body,
                    color = colors.text2,
                )
                SourceLine(source)
                TetherTextButton(
                    label = stringResource(R.string.executors_install),
                    onClick = onInstall,
                )
            }
            is LinuxState.SettingUp -> SettingUpBody(state)
            LinuxState.Ready -> {
                Text(
                    text = stringResource(R.string.executors_linux_ready_body),
                    style = body,
                    color = colors.text2,
                )
                ReadyActions(onRepair = onRepair, onRemove = onRemove)
            }
            is LinuxState.Broken -> BrokenBody(state = state, source = source, onRepair = onRepair)
        }
    }
}

/** The step line and progress bar of a running setup. */
@Composable
private fun SettingUpBody(state: LinuxState.SettingUp) {
    val body = MaterialTheme.typography.bodySmall
    Text(text = setupStepText(state), style = body, color = TetherTheme.colors.text2)
    if (state.total > 0L) {
        LinearProgressIndicator(
            progress = { downloadFraction(state.downloaded, state.total) },
            modifier = Modifier.fillMaxWidth(),
        )
    } else {
        LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
    }
}

/** The failure, and Repair when this build can run a setup. Without Repair, no advice is given. */
@Composable
private fun BrokenBody(
    state: LinuxState.Broken,
    source: String?,
    onRepair: (() -> Unit)?,
) {
    val body = MaterialTheme.typography.bodySmall
    Text(text = state.failure.text(), style = body, color = TetherTheme.colors.text2)
    if (onRepair != null) {
        Text(
            text = stringResource(R.string.executors_linux_broken_body),
            style = body,
            color = TetherTheme.colors.text2,
        )
        SourceLine(source)
        PrimaryButton(label = stringResource(R.string.executors_linux_repair), onClick = onRepair)
    }
}

/** Repair and, when the build offers it, Remove. */
@Composable
private fun ReadyActions(
    onRepair: (() -> Unit)?,
    onRemove: (() -> Unit)?,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        onRepair?.let {
            TetherTextButton(label = stringResource(R.string.executors_linux_repair), onClick = it)
        }
        onRemove?.let {
            TetherTextButton(
                label = stringResource(R.string.executors_linux_remove),
                onClick = it,
                tone = TextButtonTone.Danger,
            )
        }
    }
}

/** The one-line headline for each state. */
@Composable
private fun linuxHeadline(state: LinuxState): String =
    when (state) {
        LinuxState.Loading -> stringResource(R.string.executors_linux_checking)
        is LinuxState.Unavailable -> stringResource(R.string.executors_not_available)
        LinuxState.NotInstalled -> stringResource(R.string.executors_linux_not_installed)
        is LinuxState.SettingUp -> setupStepText(state)
        LinuxState.Ready -> stringResource(R.string.executors_linux_ready)
        is LinuxState.Broken -> stringResource(R.string.executors_linux_broken)
    }

/** The setup step for the byte counts so far: starting, downloading, or setting up. */
@Composable
private fun setupStepText(state: LinuxState.SettingUp): String =
    when (linuxPhaseOf(state.downloaded, state.total)) {
        LinuxPhase.Starting -> stringResource(R.string.executors_linux_starting)
        LinuxPhase.Downloading ->
            if (state.total > 0L) {
                stringResource(
                    R.string.executors_linux_downloading,
                    sizeText(state.downloaded),
                    sizeText(state.total),
                )
            } else {
                stringResource(R.string.executors_linux_downloading_unknown)
            }
        LinuxPhase.SettingUp -> stringResource(R.string.executors_linux_setting_up)
    }

/** The status icon for each state, or null when the state has none. */
private fun linuxMark(state: LinuxState): SessionState? =
    when (state) {
        LinuxState.Loading -> null
        is LinuxState.Unavailable -> null
        LinuxState.NotInstalled -> SessionState.Idle
        is LinuxState.SettingUp -> SessionState.Running
        LinuxState.Ready -> SessionState.Done
        is LinuxState.Broken -> SessionState.Failed
    }
