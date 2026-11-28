package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace

/** The fraction of the window width that a toast may fill. */
private const val TOAST_MAX_WIDTH_FRACTION = 0.8f

/**
 * Hosts toasts for [hostState]: a short message with no action, such as "Copied". A toast is a
 * pill on the raised surface of the app theme with a hairline border, so it follows Light and
 * Dark. The caller places the host at the top of the screen, below the status bar. A message with
 * an action is a snackbar, which [TetherSnackbarHost] shows at the bottom.
 */
@Composable
fun TetherToastHost(
    hostState: SnackbarHostState,
    modifier: Modifier = Modifier,
) {
    SnackbarHost(hostState = hostState, modifier = modifier) { data ->
        Toast(message = data.visuals.message)
    }
}

@Composable
private fun Toast(message: String) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.full)
    BoxWithConstraints(
        modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space3),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .widthIn(max = maxWidth * TOAST_MAX_WIDTH_FRACTION)
                    .clip(shape)
                    .background(colors.surfaceRaised)
                    .border(TetherBorder.hairline, colors.lineInput, shape)
                    .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = message,
                style = MaterialTheme.typography.labelMedium,
                color = colors.text,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
            )
        }
    }
}
