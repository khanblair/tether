package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherInputIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherChip
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherIconButton
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

// The send disc is 40 dp inside a 48 dp touch target. No token has that size yet (see the report).
private val SEND_DISC = 40.dp

/**
 * The row under the field: plus, mode chip, model chip, a spacer, voice, and send or stop. A null
 * [onOpenMode] or [onOpenModel] draws the chip without a click.
 */
@Suppress("LongParameterList")
@Composable
internal fun ComposerBar(
    mode: SessionMode,
    modelLabel: String,
    running: Boolean,
    canSend: Boolean,
    onPlus: () -> Unit,
    onOpenMode: (() -> Unit)?,
    onOpenModel: (() -> Unit)?,
    onVoice: () -> Unit,
    onStop: () -> Unit,
    onSend: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space0_5),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        TetherIconButton(
            icon = TetherIcons.Plus,
            contentDescription = stringResource(R.string.compose_plus_title),
            onClick = onPlus,
        )
        TetherChip(
            label = stringResource(mode.labelRes()),
            tone = modeChipTone(mode),
            leadingIcon = modeIcon(mode),
            onClick = onOpenMode,
        )
        TetherChip(
            label = modelLabel,
            trailingChevron = true,
            onClick = onOpenModel,
        )
        Spacer(Modifier.weight(1f))
        TetherIconButton(
            icon = TetherInputIcons.Mic,
            contentDescription = stringResource(R.string.composer_voice_input),
            onClick = onVoice,
        )
        if (running) {
            SendDisc(
                icon = TetherInputIcons.Square,
                description = stringResource(R.string.composer_stop),
                background = TetherTheme.colors.text,
                tint = TetherTheme.colors.bg,
                enabled = true,
                onClick = onStop,
            )
        } else {
            val colors = TetherTheme.colors
            SendDisc(
                icon = TetherInputIcons.ArrowUp,
                description = stringResource(R.string.composer_send),
                background = if (canSend) colors.accent else colors.surfaceSunken,
                tint = if (canSend) colors.onAccent else colors.textDisabled,
                enabled = canSend,
                onClick = onSend,
            )
        }
    }
}

/** The mode icon: a zap for Auto, a shield with an alert for Bypass, and a shield otherwise. */
private fun modeIcon(mode: SessionMode): ImageVector =
    when (mode) {
        SessionMode.AUTO -> TetherIcons.Zap
        SessionMode.BYPASS -> TetherInputIcons.ShieldAlert
        SessionMode.PLAN, SessionMode.ASK, SessionMode.ACCEPT_EDITS -> TetherInputIcons.Shield
    }

/** A 40 dp filled disc with an icon, inside a 48 dp touch target. */
@Suppress("LongParameterList")
@Composable
private fun SendDisc(
    icon: ImageVector,
    description: String,
    background: Color,
    tint: Color,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    IconButton(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier.size(TetherSize.touchMin),
    ) {
        Box(
            modifier = Modifier.size(SEND_DISC).clip(CircleShape).background(background),
            contentAlignment = Alignment.Center,
        ) {
            TetherIcon(
                icon = icon,
                contentDescription = description,
                tint = tint,
                size = TetherSize.iconS,
            )
        }
    }
}
