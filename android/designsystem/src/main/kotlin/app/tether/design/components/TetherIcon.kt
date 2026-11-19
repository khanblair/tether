package app.tether.design.components

import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize

/**
 * A Lucide icon at token size. A null [contentDescription] marks the icon as decorative.
 */
@Composable
fun TetherIcon(
    icon: ImageVector,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    tint: Color = TetherTheme.colors.text2,
    size: Dp = TetherSize.icon,
) {
    Icon(
        imageVector = icon,
        contentDescription = contentDescription,
        modifier = modifier.size(size),
        tint = tint,
    )
}
