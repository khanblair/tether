package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.material3.IconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.Copy
import app.tether.design.Pencil
import app.tether.design.RefreshCw
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The small icon row under a message. A null action is left out. Each icon is a 16 dp glyph in a
 * 36 dp button, spaced by 4 dp. The 36 dp target is an exception to the 48 dp rule for this row
 * only: the actions are secondary and sit beside each other. [copyLabel] names the copy action
 * for TalkBack.
 */
@Composable
internal fun MessageActionRow(
    copyLabel: String,
    modifier: Modifier = Modifier,
    onCopy: (() -> Unit)? = null,
    onEdit: (() -> Unit)? = null,
    onRegenerate: (() -> Unit)? = null,
    alignEnd: Boolean = false,
) {
    val editLabel = stringResource(R.string.chat_edit_message)
    val regenerateLabel = stringResource(R.string.chat_regenerate_reply)
    Row(
        horizontalArrangement =
            Arrangement.spacedBy(
                TetherSpace.space1,
                if (alignEnd) Alignment.End else Alignment.Start,
            ),
        modifier = modifier.fillMaxWidth(),
    ) {
        onCopy?.let { ActionIcon(TetherIcons.Copy, copyLabel, it) }
        onEdit?.let { ActionIcon(TetherIcons.Pencil, editLabel, it) }
        onRegenerate?.let { ActionIcon(TetherIcons.RefreshCw, regenerateLabel, it) }
    }
}

@Composable
private fun ActionIcon(
    icon: ImageVector,
    label: String,
    onClick: () -> Unit,
) {
    IconButton(onClick = onClick, modifier = Modifier.size(TetherSize.touchCompact)) {
        TetherIcon(
            icon = icon,
            contentDescription = label,
            tint = TetherTheme.colors.text3,
            size = TetherSize.iconXs,
        )
    }
}
