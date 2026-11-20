package app.tether.design.components

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import app.tether.design.TetherIcons
import app.tether.design.ThemeMode

/**
 * The theme toggle: an icon button that shows the current [mode] (monitor, sun, or moon). The
 * caller passes [contentDescription], which names the current choice and the next one.
 */
@Composable
fun ThemeToggleButton(
    mode: ThemeMode,
    contentDescription: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    TetherIconButton(
        icon = mode.icon(),
        contentDescription = contentDescription,
        onClick = onClick,
        modifier = modifier,
    )
}

private fun ThemeMode.icon(): ImageVector =
    when (this) {
        ThemeMode.System -> TetherIcons.Monitor
        ThemeMode.Light -> TetherIcons.Sun
        ThemeMode.Dark -> TetherIcons.Moon
    }
