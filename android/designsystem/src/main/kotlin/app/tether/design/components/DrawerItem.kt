package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A destination in the navigation drawer. The selected item sits on the tonal fill. An optional
 * [badge] is plain text at the end, such as the count in Needs attention.
 */
@Composable
fun DrawerItem(
    label: String,
    icon: ImageVector,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    badge: String? = null,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val colors = TetherTheme.colors
    val background: Color =
        when {
            selected -> colors.tonal
            pressed -> colors.surfaceSunken
            else -> Color.Transparent
        }
    val content = if (selected) colors.onTonal else colors.text
    val secondary = if (selected) colors.onTonal else colors.text2

    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space2)
                .heightIn(min = TetherSize.touchMin)
                .clip(RoundedCornerShape(TetherRadius.sm))
                .background(background)
                .clickable(
                    interactionSource = interactionSource,
                    role = Role.Tab,
                    onClick = onClick,
                ).semantics(mergeDescendants = true) { this.selected = selected }
                .padding(horizontal = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(icon = icon, contentDescription = null, tint = secondary)
        Spacer(Modifier.width(TetherSpace.space3))
        Text(
            text = label,
            style = MaterialTheme.typography.bodyLarge,
            color = content,
            modifier = Modifier.weight(1f),
        )
        if (badge != null) {
            Spacer(Modifier.width(TetherSpace.space2))
            Text(
                text = badge,
                style = MaterialTheme.typography.labelMedium,
                color = secondary,
            )
        }
    }
}
