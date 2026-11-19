package app.tether.design.components

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A low-emphasis text button with an optional leading [icon]. Its height is at least the
 * minimum touch target.
 */
@Composable
fun TetherTextButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    tone: TextButtonTone = TextButtonTone.Neutral,
    icon: ImageVector? = null,
    enabled: Boolean = true,
) {
    val color: Color =
        if (tone == TextButtonTone.Danger) TetherTheme.colors.danger else TetherTheme.colors.text2
    TextButton(
        onClick = onClick,
        modifier = modifier.heightIn(min = TetherSize.touchMin),
        enabled = enabled,
        colors =
            ButtonDefaults.textButtonColors(
                contentColor = color,
                disabledContentColor = TetherTheme.colors.textDisabled,
            ),
    ) {
        if (icon != null) {
            TetherIcon(
                icon = icon,
                contentDescription = null,
                tint = color,
                size = TetherSize.iconXs,
            )
            Spacer(Modifier.width(TetherSpace.space2))
        }
        Text(text = label, style = MaterialTheme.typography.labelLarge)
    }
}
