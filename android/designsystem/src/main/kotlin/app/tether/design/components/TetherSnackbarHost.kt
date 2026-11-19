package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarData
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

/** The fraction of the window width that a toast may fill. */
private const val TOAST_MAX_WIDTH_FRACTION = 0.8f

/**
 * Hosts snackbars and toasts for [hostState]. A message with an action shows as a snackbar; one
 * without shows as a centered toast. Both use the inverse surface of the app theme.
 */
@Composable
fun TetherSnackbarHost(
    hostState: SnackbarHostState,
    modifier: Modifier = Modifier,
) {
    SnackbarHost(hostState = hostState, modifier = modifier) { data ->
        TetherSnackbar(data)
    }
}

@Composable
private fun TetherSnackbar(data: SnackbarData) {
    val actionLabel = data.visuals.actionLabel
    if (actionLabel == null) {
        Toast(message = data.visuals.message)
    } else {
        ActionSnackbar(
            message = data.visuals.message,
            actionLabel = actionLabel,
            onAction = data::performAction,
        )
    }
}

@Composable
private fun ActionSnackbar(
    message: String,
    actionLabel: String,
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
        TextButton(
            onClick = onAction,
            modifier = Modifier.heightIn(min = TetherSize.touchMin),
            colors = ButtonDefaults.textButtonColors(contentColor = colors.tonal),
        ) {
            Text(text = actionLabel, style = MaterialTheme.typography.labelLarge)
        }
    }
}

@Composable
private fun Toast(message: String) {
    val colors = TetherTheme.colors
    BoxWithConstraints(
        modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space3),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .widthIn(max = maxWidth * TOAST_MAX_WIDTH_FRACTION)
                    .clip(RoundedCornerShape(TetherRadius.full))
                    .background(colors.text)
                    .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = message,
                style = MaterialTheme.typography.labelMedium,
                color = colors.bg,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
            )
        }
    }
}
