package app.tether.design.components

import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize

/** The one primary action on a screen. Plum in light mode, mint in dark mode. */
@Composable
fun PrimaryButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    loading: Boolean = false,
) {
    Button(
        onClick = onClick,
        modifier = modifier.heightIn(min = TetherSize.touchMin),
        enabled = enabled && !loading,
        shape = RoundedCornerShape(TetherRadius.sm),
        colors =
            ButtonDefaults.buttonColors(
                containerColor = TetherTheme.colors.accent,
                contentColor = TetherTheme.colors.onAccent,
                disabledContainerColor = TetherTheme.colors.surfaceSunken,
                disabledContentColor = TetherTheme.colors.textDisabled,
            ),
    ) {
        if (loading) {
            CircularProgressIndicator(
                modifier = Modifier.size(TetherSize.iconXs),
                color = TetherTheme.colors.onAccent,
                strokeWidth = TetherBorder.inputFocus,
            )
        } else {
            Text(text = label, style = MaterialTheme.typography.labelLarge)
        }
    }
}
