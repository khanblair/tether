package app.tether.design.components

import androidx.compose.foundation.layout.size
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize

/**
 * An icon-only button with a minimum touch target. [contentDescription] names the action for
 * TalkBack. A disabled button shows the icon in the disabled text color.
 */
@Composable
fun TetherIconButton(
    icon: ImageVector,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    tint: Color = TetherTheme.colors.text2,
) {
    IconButton(
        onClick = onClick,
        modifier = modifier.size(TetherSize.touchMin),
        enabled = enabled,
    ) {
        TetherIcon(
            icon = icon,
            contentDescription = contentDescription,
            tint = if (enabled) tint else TetherTheme.colors.textDisabled,
        )
    }
}
