package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * Hosts snackbars for [hostState]: a message with an action, such as Undo. It uses the inverse
 * surface of the app theme and sits at the bottom of the screen. A message without an action is a
 * toast, which [TetherToastHost] shows at the top.
 */
@Composable
fun TetherSnackbarHost(
    hostState: SnackbarHostState,
    modifier: Modifier = Modifier,
) {
    SnackbarHost(hostState = hostState, modifier = modifier) { data ->
        ActionSnackbar(
            message = data.visuals.message,
            actionLabel = data.visuals.actionLabel,
            onAction = data::performAction,
        )
    }
}

@Composable
private fun ActionSnackbar(
    message: String,
    actionLabel: String?,
    onAction: () -> Unit,
) {
    val colors = TetherTheme.colors
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space3)
                .heightIn(min = TetherSize.touchMin)
                .clip(RoundedCornerShape(TetherRadius.sm))
                .background(colors.text)
                .padding(start = TetherSpace.space4, end = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.bg,
            modifier =
                Modifier
                    .weight(1f)
                    .semantics { liveRegion = LiveRegionMode.Polite },
        )
        if (actionLabel != null) {
            TextButton(
                onClick = onAction,
                modifier = Modifier.heightIn(min = TetherSize.touchMin),
                colors = ButtonDefaults.textButtonColors(contentColor = colors.tonal),
            ) {
                Text(text = actionLabel, style = MaterialTheme.typography.labelLarge)
            }
        }
    }
}
