package app.tether.design.components

import android.provider.Settings
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherMotion
import app.tether.design.tokens.TetherSize

/**
 * The status icon for a session. It is decorative: the row text carries the meaning. A running
 * mark turns slowly unless the system animation scale is zero.
 */
@Composable
fun StatusMark(
    state: SessionState,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    when (state) {
        SessionState.Running -> RunningMark(modifier, colors.text2)
        SessionState.Done -> StaticMark(TetherIcons.CircleCheck, colors.success, modifier)
        SessionState.Waiting -> StaticMark(TetherIcons.CirclePause, colors.warning, modifier)
        SessionState.Failed -> StaticMark(TetherIcons.CircleX, colors.danger, modifier)
        SessionState.Idle -> StaticMark(TetherIcons.CircleDot, colors.text3, modifier)
    }
}

@Composable
private fun RunningMark(
    modifier: Modifier,
    tint: Color,
) {
    val transition = rememberInfiniteTransition(label = "statusMarkSpin")
    val angle by
        transition.animateFloat(
            initialValue = 0f,
            targetValue = FULL_TURN,
            animationSpec =
                infiniteRepeatable(
                    animation =
                        tween(
                            durationMillis = TetherMotion.streamCaretMs,
                            easing = LinearEasing,
                        ),
                    repeatMode = RepeatMode.Restart,
                ),
            label = "statusMarkAngle",
        )
    val spin = animationsEnabled()
    StaticMark(
        icon = TetherIcons.Loader,
        tint = tint,
        modifier = modifier.rotate(if (spin) angle else 0f),
    )
}

@Composable
private fun StaticMark(
    icon: ImageVector,
    tint: Color,
    modifier: Modifier,
) {
    TetherIcon(
        icon = icon,
        contentDescription = null,
        modifier = modifier,
        tint = tint,
        size = TetherSize.iconS,
    )
}

@Composable
private fun animationsEnabled(): Boolean {
    val context = LocalContext.current
    return remember(context) {
        runCatching {
            Settings.Global.getFloat(
                context.contentResolver,
                Settings.Global.ANIMATOR_DURATION_SCALE,
                1f,
            ) > 0f
        }.getOrDefault(true)
    }
}

private const val FULL_TURN = 360f
