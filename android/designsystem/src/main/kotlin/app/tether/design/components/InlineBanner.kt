package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherColors
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A persistent notice inside a screen. The polite live region announces a new result once.
 * An optional [actionLabel] with [onAction] adds a low-emphasis button on the right.
 */
@Composable
fun InlineBanner(
    text: String,
    tone: BannerTone,
    modifier: Modifier = Modifier,
    title: String? = null,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    val colors = TetherTheme.colors
    val strong = strongColor(tone, colors)

    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(TetherRadius.md))
                .background(softColor(tone, colors))
                .semantics { liveRegion = LiveRegionMode.Polite }
                .padding(TetherSpace.space4),
        verticalAlignment = Alignment.Top,
    ) {
        TetherIcon(
            icon = toneIcon(tone),
            contentDescription = null,
            tint = strong,
            size = TetherSize.iconS,
        )
        Spacer(Modifier.width(TetherSpace.space3))
        Column(modifier = Modifier.weight(1f)) {
            if (title != null) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleSmall,
                    color = strong,
                )
            }
            Text(
                text = text,
                style = MaterialTheme.typography.bodySmall,
                color = colors.text,
            )
        }
        if (actionLabel != null && onAction != null) {
            Spacer(Modifier.width(TetherSpace.space2))
            TetherTextButton(label = actionLabel, onClick = onAction)
        }
    }
}

private fun softColor(
    tone: BannerTone,
    colors: TetherColors,
): Color =
    when (tone) {
        BannerTone.Info -> colors.surfaceSunken
        BannerTone.Success -> colors.successSoft
        BannerTone.Warning -> colors.warningSoft
        BannerTone.Danger -> colors.dangerSoft
    }

private fun strongColor(
    tone: BannerTone,
    colors: TetherColors,
): Color =
    when (tone) {
        BannerTone.Info -> colors.info
        BannerTone.Success -> colors.success
        BannerTone.Warning -> colors.warning
        BannerTone.Danger -> colors.danger
    }

private fun toneIcon(tone: BannerTone): ImageVector =
    when (tone) {
        BannerTone.Success -> TetherIcons.CircleCheck
        BannerTone.Danger -> TetherIcons.CircleX
        BannerTone.Info, BannerTone.Warning -> TetherIcons.CircleDot
    }
