package app.tether.session

import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherInputIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet

/**
 * The plus sheet "Add to message": four rows for what the composer can take. [onPick] runs the
 * row's action. The caller closes the sheet.
 */
@Composable
internal fun PlusSheet(
    draft: String,
    supportsImages: Boolean,
    onDismiss: () -> Unit,
    onPick: (PlusRow) -> Unit,
    modifier: Modifier = Modifier,
) {
    TetherSheet(
        title = stringResource(R.string.compose_plus_title),
        onDismiss = onDismiss,
        modifier = modifier,
    ) {
        PlusRowItem(PlusRow.PHOTO, TetherInputIcons.Image, draft, supportsImages, onPick)
        PlusRowItem(PlusRow.CAMERA, TetherInputIcons.Camera, draft, supportsImages, onPick)
        PlusRowItem(PlusRow.FILE, TetherInputIcons.Paperclip, draft, supportsImages, onPick)
        PlusRowItem(PlusRow.MENTION, TetherInputIcons.AtSign, draft, supportsImages, onPick)
        PlusRowItem(PlusRow.COMMANDS, TetherInputIcons.SquareSlash, draft, supportsImages, onPick)
    }
}

/** One row of the plus sheet. A row that is off shows the reason in place of its description. */
@Composable
private fun PlusRowItem(
    row: PlusRow,
    icon: ImageVector,
    draft: String,
    supportsImages: Boolean,
    onPick: (PlusRow) -> Unit,
) {
    val block = plusRowBlock(row, supportsImages, draft)
    val reason = block?.let { stringResource(it.labelRes) }
    val colors = TetherTheme.colors
    ListRow(
        title = stringResource(plusTitleRes(row)),
        subtitle = reason ?: stringResource(plusDescriptionRes(row)),
        leading = {
            TetherIcon(
                icon = icon,
                contentDescription = null,
                tint = if (block == null) colors.text2 else colors.textDisabled,
            )
        },
        enabled = block == null,
        onClick = if (block == null) ({ onPick(row) }) else null,
    )
}

@StringRes
private fun plusTitleRes(row: PlusRow): Int =
    when (row) {
        PlusRow.PHOTO -> R.string.compose_plus_photo
        PlusRow.CAMERA -> R.string.compose_plus_camera
        PlusRow.FILE -> R.string.compose_plus_file
        PlusRow.MENTION -> R.string.compose_plus_mention
        PlusRow.COMMANDS -> R.string.compose_plus_commands
    }

@StringRes
private fun plusDescriptionRes(row: PlusRow): Int =
    when (row) {
        PlusRow.PHOTO -> R.string.compose_plus_photo_sub
        PlusRow.CAMERA -> R.string.compose_plus_camera_sub
        PlusRow.FILE -> R.string.compose_plus_file_sub
        PlusRow.MENTION -> R.string.compose_plus_mention_sub
        PlusRow.COMMANDS -> R.string.compose_plus_commands_sub
    }
