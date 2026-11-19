package app.tether.design.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A full-width outlined button that starts a connection test. While [TestState.Testing] it shows
 * a spinner and cannot be pressed. [label] is the text for the current state, chosen by the caller.
 */
@Composable
fun TestConnectionButton(
    state: TestState,
    label: String,
    onTest: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val clickable = enabled && state != TestState.Testing
    val colors = TetherTheme.colors
    OutlinedButton(
        onClick = onTest,
        modifier = modifier.fillMaxWidth().heightIn(min = TetherSize.touchMin),
        enabled = clickable,
        shape = RoundedCornerShape(TetherRadius.sm),
        border =
            BorderStroke(
                width = TetherBorder.input,
                color = if (clickable) colors.lineInput else colors.line,
            ),
        colors =
            ButtonDefaults.outlinedButtonColors(
                contentColor = colors.text,
                disabledContentColor = colors.textDisabled,
            ),
    ) {
        if (state == TestState.Testing) {
            CircularProgressIndicator(
                modifier = Modifier.size(TetherSize.iconXs),
                color = colors.text,
                strokeWidth = TetherBorder.inputFocus,
            )
        } else {
            TetherIcon(
                icon = TetherIcons.Zap,
                contentDescription = null,
                tint = colors.text,
                size = TetherSize.iconXs,
            )
        }
        Spacer(Modifier.width(TetherSpace.space2))
        Text(text = label, style = MaterialTheme.typography.labelLarge)
    }
}
