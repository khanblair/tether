package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A project in the project list: a folder icon, the name, the branch, and a one-line summary.
 * An optional [badge] sits at the trailing edge. Each line truncates with an ellipsis.
 */
@Composable
fun ProjectRow(
    name: String,
    branch: String,
    summary: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    badge: (@Composable () -> Unit)? = null,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val colors = TetherTheme.colors

    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.approvalBarMin)
                .background(if (pressed) colors.surfaceSunken else Color.Transparent)
                .clickable(
                    interactionSource = interactionSource,
                    role = Role.Button,
                    onClick = onClick,
                ).semantics(mergeDescendants = true) {}
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(
            icon = TetherIcons.Folder,
            contentDescription = null,
            tint = colors.text2,
        )
        Spacer(Modifier.width(TetherSpace.space3))
        Column(modifier = Modifier.weight(1f)) {
            SingleLine(name, MaterialTheme.typography.bodyLarge, colors.text)
            SingleLine(branch, TetherTheme.monoSmall, colors.text2)
            SingleLine(summary, MaterialTheme.typography.bodySmall, colors.text2)
        }
        if (badge != null) {
            Spacer(Modifier.width(TetherSpace.space3))
            badge()
        }
    }
}

@Composable
private fun SingleLine(
    text: String,
    style: TextStyle,
    color: Color,
) {
    Text(
        text = text,
        style = style,
        color = color,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
    )
}
