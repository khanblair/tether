package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
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
import androidx.compose.ui.text.style.TextOverflow
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A list row with an optional leading and trailing slot. It is clickable when [onClick] is set,
 * and a pressed row turns the sunken surface color. [onLongClick] needs [onClick] to be set too.
 * A [danger] row shows its title in the danger color, unless it is disabled. The title and the
 * subtitle wrap without limit unless [titleMaxLines] or [subtitleMaxLines] is set, and then they
 * end in an ellipsis.
 */
@Composable
fun ListRow(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    leading: (@Composable () -> Unit)? = null,
    trailing: (@Composable () -> Unit)? = null,
    onClick: (() -> Unit)? = null,
    enabled: Boolean = true,
    onLongClick: (() -> Unit)? = null,
    danger: Boolean = false,
    titleMaxLines: Int = Int.MAX_VALUE,
    subtitleMaxLines: Int = Int.MAX_VALUE,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val pressed by interactionSource.collectIsPressedAsState()
    val isPressed = pressed && enabled && onClick != null
    val pressedBackground =
        if (isPressed) TetherTheme.colors.surfaceSunken else Color.Transparent
    val clickModifier =
        if (onClick != null) {
            Modifier.combinedClickable(
                interactionSource = interactionSource,
                enabled = enabled,
                role = Role.Button,
                onClick = onClick,
                onLongClick = onLongClick,
            )
        } else {
            Modifier
        }
    val titleColor =
        when {
            !enabled -> TetherTheme.colors.textDisabled
            danger -> TetherTheme.colors.danger
            else -> TetherTheme.colors.text
        }

    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.topBar)
                .background(pressedBackground)
                .then(clickModifier)
                .semantics(mergeDescendants = true) {}
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (leading != null) {
            leading()
            Spacer(Modifier.width(TetherSpace.space3))
        }
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyLarge,
                color = titleColor,
                maxLines = titleMaxLines,
                overflow = TextOverflow.Ellipsis,
            )
            if (subtitle != null) {
                Text(
                    text = subtitle,
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text2,
                    maxLines = subtitleMaxLines,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
        if (trailing != null) {
            Spacer(Modifier.width(TetherSpace.space3))
            trailing()
        }
    }
}
