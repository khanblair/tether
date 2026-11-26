package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

// The prototype's chip is 36 px tall; no token has that height yet (see the task report).
private val CHIP_MIN_HEIGHT = 36.dp
private val DANGER_BORDER = 1.5.dp

private class ChipPalette(
    val background: Color,
    val content: Color,
    val border: Color,
    val borderWidth: Dp,
)

/**
 * A small pill for a setting, such as the mode or the model. [tone] sets the fill and outline. A
 * null [onClick] draws the chip without a click. The touch target is at least 48 dp tall.
 */
@Composable
fun TetherChip(
    label: String,
    modifier: Modifier = Modifier,
    tone: ChipTone = ChipTone.Neutral,
    leadingIcon: ImageVector? = null,
    trailingChevron: Boolean = false,
    onClick: (() -> Unit)? = null,
) {
    val palette = chipPalette(tone)
    val shape = RoundedCornerShape(TetherRadius.sm)
    val clickable =
        if (onClick != null) {
            Modifier.clickable(role = Role.Button, onClick = onClick)
        } else {
            Modifier
        }
    Box(
        modifier = modifier.minimumInteractiveComponentSize().then(clickable),
        contentAlignment = Alignment.Center,
    ) {
        Row(
            modifier =
                Modifier
                    .defaultMinSize(minHeight = CHIP_MIN_HEIGHT)
                    .clip(shape)
                    .background(palette.background, shape)
                    .border(palette.borderWidth, palette.border, shape)
                    .padding(horizontal = TetherSpace.space3),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1),
        ) {
            leadingIcon?.let {
                TetherIcon(it, null, tint = palette.content, size = TetherSize.iconXs)
            }
            Text(
                text = label,
                style = MaterialTheme.typography.labelLarge,
                color = palette.content,
                maxLines = 1,
            )
            if (trailingChevron) {
                TetherIcon(
                    icon = TetherIcons.ChevronDown,
                    contentDescription = null,
                    tint = palette.content,
                    size = TetherSize.iconXs,
                )
            }
        }
    }
}

@Composable
private fun chipPalette(tone: ChipTone): ChipPalette {
    val colors = TetherTheme.colors
    val hairline = TetherBorder.hairline
    return when (tone) {
        ChipTone.Neutral ->
            ChipPalette(Color.Transparent, colors.text2, colors.lineInput, hairline)
        ChipTone.Accent ->
            ChipPalette(colors.accent, colors.onAccent, Color.Transparent, hairline)
        ChipTone.Danger ->
            ChipPalette(Color.Transparent, colors.danger, colors.danger, DANGER_BORDER)
    }
}
