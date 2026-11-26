package app.tether.session.voice

import androidx.annotation.StringRes
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherEase
import app.tether.design.tokens.TetherMotion
import app.tether.design.tokens.TetherSpace
import app.tether.git.rememberReducedMotion

// The bar size and the wave row height have no tokens yet (see the report).
private val BAR_WIDTH = 5.dp
private val BAR_RADIUS = 3.dp
private val WAVE_HEIGHT = 48.dp

/** The level every bar shows when the system has animations turned off. */
private const val REDUCED_LEVEL = 0.5f

/** The "Listening" sheet. [state] decides what the body shows. */
@Composable
internal fun ListeningSheet(
    state: VoiceSheetState,
    onAllow: () -> Unit,
    onTryAgain: () -> Unit,
    onInsert: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
) {
    TetherSheet(
        title = stringResource(R.string.voice_sheet_title),
        onDismiss = onCancel,
        modifier = modifier,
    ) {
        ListeningBody(
            state = state,
            onAllow = onAllow,
            onTryAgain = onTryAgain,
            onInsert = onInsert,
            onCancel = onCancel,
        )
    }
}

/** The sheet body for each state. It has no sheet frame, so previews can draw it. */
@Composable
internal fun ListeningBody(
    state: VoiceSheetState,
    onAllow: () -> Unit,
    onTryAgain: () -> Unit,
    onInsert: () -> Unit,
    onCancel: () -> Unit,
) {
    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space4),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        when (state) {
            VoiceSheetState.Requesting -> ListeningContent("", emptyList(), onInsert, onCancel)
            is VoiceSheetState.Listening ->
                ListeningContent(state.transcript, state.levels, onInsert, onCancel)
            is VoiceSheetState.Denied -> {
                val allow =
                    if (state.canAskAgain) {
                        R.string.voice_allow_mic
                    } else {
                        R.string.voice_open_settings
                    }
                MessageContent(R.string.voice_denied, allow, onAllow, onCancel)
            }
            VoiceSheetState.Unavailable ->
                MessageContent(R.string.voice_unavailable, null, onCancel, onCancel)
            is VoiceSheetState.Failed ->
                MessageContent(
                    failureRes(state.reason),
                    R.string.voice_try_again,
                    onTryAgain,
                    onCancel,
                )
        }
    }
}

/** The waveform, the live transcript, and the Cancel and Insert text buttons. */
@Composable
private fun ListeningContent(
    transcript: String,
    levels: List<Float>,
    onInsert: () -> Unit,
    onCancel: () -> Unit,
) {
    val reduced = rememberReducedMotion()
    WaveBars(if (reduced) List(VOICE_BARS) { REDUCED_LEVEL } else levels, animate = !reduced)
    val empty = transcript.isEmpty()
    Text(
        text = if (empty) stringResource(R.string.voice_hint) else transcript,
        style = MaterialTheme.typography.bodyLarge,
        color = if (empty) TetherTheme.colors.text3 else TetherTheme.colors.text,
        textAlign = TextAlign.Center,
        modifier = Modifier.fillMaxWidth(),
    )
    ButtonRow(R.string.voice_insert, transcript.isNotBlank(), onInsert, onCancel)
}

/** A message with one primary action. A null [primary] shows Cancel only. */
@Composable
private fun MessageContent(
    @StringRes message: Int,
    @StringRes primary: Int?,
    onPrimary: () -> Unit,
    onCancel: () -> Unit,
) {
    Text(
        text = stringResource(message),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text,
        textAlign = TextAlign.Center,
        modifier = Modifier.fillMaxWidth(),
    )
    ButtonRow(primary, enabled = true, onPrimary = onPrimary, onCancel = onCancel)
}

/** Cancel as plain text, then the primary action when there is one. */
@Composable
private fun ButtonRow(
    @StringRes primary: Int?,
    enabled: Boolean,
    onPrimary: () -> Unit,
    onCancel: () -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherTextButton(label = stringResource(R.string.voice_cancel), onClick = onCancel)
        if (primary != null) {
            Spacer(Modifier.width(TetherSpace.space2))
            PrimaryButton(
                label = stringResource(primary),
                onClick = onPrimary,
                enabled = enabled,
            )
        }
    }
}

/** One bar per level. Each bar eases to its new height unless animations are off. */
@Composable
private fun WaveBars(
    levels: List<Float>,
    animate: Boolean,
) {
    Row(
        modifier = Modifier.height(WAVE_HEIGHT),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        levels.forEach { level -> Bar(barHeight(level), animate) }
    }
}

@Composable
private fun Bar(
    height: Dp,
    animate: Boolean,
) {
    val duration = if (animate) TetherMotion.fastMs else TetherMotion.instantMs
    val shown by animateDpAsState(
        targetValue = height,
        animationSpec = tween(durationMillis = duration, easing = TetherEase.standard),
    )
    Box(
        modifier =
            Modifier
                .width(BAR_WIDTH)
                .height(shown)
                .clip(RoundedCornerShape(BAR_RADIUS))
                .background(TetherTheme.colors.accentInk),
    )
}

@StringRes
private fun failureRes(reason: VoiceFailure): Int =
    when (reason) {
        VoiceFailure.NoMatch -> R.string.voice_fail_no_match
        VoiceFailure.Network -> R.string.voice_fail_network
        VoiceFailure.Busy -> R.string.voice_fail_busy
        VoiceFailure.PermissionDenied,
        VoiceFailure.Unavailable,
        VoiceFailure.Other,
        -> R.string.voice_fail_other
    }
