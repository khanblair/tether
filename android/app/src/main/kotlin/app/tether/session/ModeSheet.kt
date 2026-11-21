package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The permission mode picker. Bypass opens a confirmation first, and the biometric check runs
 * from the confirmation's button, not from the list.
 */
@Composable
fun ModeSheet(
    state: ModeSheetState,
    running: Boolean,
    onChoose: (SessionMode) -> Unit,
    onConfirmBypass: () -> Unit,
    onCancelBypass: () -> Unit,
    onDismiss: () -> Unit,
) {
    val title =
        if (state.confirming) {
            stringResource(R.string.session_bypass_title)
        } else {
            stringResource(R.string.session_mode_title)
        }
    TetherSheet(title = title, onDismiss = onDismiss) {
        if (running) {
            InlineBanner(
                text = stringResource(R.string.session_busy_reason),
                tone = BannerTone.Info,
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
        }
        if (state.failure != null) {
            InlineBanner(
                text = stringResource(R.string.session_error_settings),
                tone = BannerTone.Danger,
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
        }
        if (state.confirming) {
            BypassConfirmation(state, onConfirmBypass, onCancelBypass)
        } else {
            val enabled = !running && !state.saving
            Column(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
                SessionMode.entries.forEach { mode ->
                    ModeRow(
                        mode = mode,
                        selected = state.current == mode,
                        enabled = enabled,
                        onClick = { onChoose(mode) },
                    )
                }
                Text(
                    text = stringResource(R.string.session_mode_scope),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                    modifier = Modifier.padding(top = TetherSpace.space2),
                )
            }
        }
    }
}

/** One mode with its one-line description. The selected mode shows a check and says so. */
@Composable
private fun ModeRow(
    mode: SessionMode,
    selected: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val isBypass = mode == SessionMode.BYPASS
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
                .semantics { this.selected = selected }
                .padding(vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = stringResource(mode.labelRes()),
                style = MaterialTheme.typography.bodyLarge,
                color =
                    when {
                        !enabled -> TetherTheme.colors.textDisabled
                        isBypass -> TetherTheme.colors.danger
                        else -> TetherTheme.colors.text
                    },
            )
            Text(
                text = stringResource(mode.descriptionRes()),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text2,
            )
        }
        if (selected) {
            Spacer(Modifier.width(TetherSpace.space3))
            TetherIcon(
                icon = TetherIcons.CircleCheck,
                contentDescription = stringResource(R.string.session_selected),
                tint = TetherTheme.colors.accent,
            )
        }
    }
}

/** The Bypass confirmation: what changes, the biometric button, and a way back. */
@Composable
private fun BypassConfirmation(
    state: ModeSheetState,
    onConfirm: () -> Unit,
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier.padding(horizontal = TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = stringResource(R.string.session_bypass_body),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.danger,
            modifier =
                Modifier
                    .fillMaxWidth()
                    .background(TetherTheme.colors.dangerSoft, RoundedCornerShape(TetherRadius.md))
                    .padding(TetherSpace.space4),
        )
        if (state.bypassDenied) {
            InlineBanner(
                text = stringResource(R.string.session_bypass_denied),
                tone = BannerTone.Warning,
            )
        }
        PrimaryButton(
            label = stringResource(R.string.session_bypass_confirm),
            onClick = onConfirm,
            modifier = Modifier.fillMaxWidth(),
            enabled = !state.saving,
            loading = state.saving,
        )
        TetherTextButton(
            label = stringResource(R.string.session_bypass_cancel),
            onClick = onCancel,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

/** The label string of a mode. Its name matches the status line. */
internal fun SessionMode.labelRes(): Int =
    when (this) {
        SessionMode.PLAN -> R.string.status_mode_plan
        SessionMode.ASK -> R.string.status_mode_ask
        SessionMode.ACCEPT_EDITS -> R.string.status_mode_accept_edits
        SessionMode.AUTO -> R.string.status_mode_auto
        SessionMode.BYPASS -> R.string.status_mode_bypass
    }

/** The one-line description of a mode, as the prototype words it. */
internal fun SessionMode.descriptionRes(): Int =
    when (this) {
        SessionMode.PLAN -> R.string.session_mode_plan_desc
        SessionMode.ASK -> R.string.session_mode_ask_desc
        SessionMode.ACCEPT_EDITS -> R.string.session_mode_accept_edits_desc
        SessionMode.AUTO -> R.string.session_mode_auto_desc
        SessionMode.BYPASS -> R.string.session_mode_bypass_desc
    }
