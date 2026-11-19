package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherColors
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A small pill label with an optional leading [icon]. The text is the meaning; the icon and
 * the fill only add emphasis.
 */
@Composable
fun StatusBadge(
    label: String,
    tone: BadgeTone,
    modifier: Modifier = Modifier,
    icon: ImageVector? = null,
) {
    val (fill, content) = badgeColors(tone, TetherTheme.colors)
    Row(
        modifier =
            modifier
                .background(fill, RoundedCornerShape(TetherRadius.full))
                .padding(horizontal = TetherSpace.space2, vertical = TetherSpace.space0_5),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (icon != null) {
            TetherIcon(
                icon = icon,
                contentDescription = null,
                tint = content,
                size = TetherSize.iconXs,
            )
            Spacer(Modifier.width(TetherSpace.space1))
        }
        Text(
            text = label,
            style = MaterialTheme.typography.labelSmall,
            color = content,
        )
    }
}

private fun badgeColors(
    tone: BadgeTone,
    colors: TetherColors,
): Pair<Color, Color> =
    when (tone) {
        BadgeTone.Neutral -> colors.surfaceSunken to colors.text2
        BadgeTone.Tonal -> colors.tonal to colors.onTonal
        BadgeTone.Success -> colors.successSoft to colors.success
        BadgeTone.Warning -> colors.warningSoft to colors.warning
        BadgeTone.Danger -> colors.dangerSoft to colors.danger
    }
