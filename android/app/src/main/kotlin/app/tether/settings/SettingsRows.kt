package app.tether.settings

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherSize

/** A row that names a feature the app cannot do yet. It does not open anything. */
@Composable
internal fun NotAvailableRow(
    title: String,
    explanation: String,
) {
    ListRow(title = title, subtitle = explanation)
}

/** The trailing chevron of a row that opens a screen. Decorative: the row carries the name. */
@Composable
internal fun ChevronTrailing() {
    TetherIcon(
        icon = TetherIcons.ChevronRight,
        contentDescription = null,
        tint = TetherTheme.colors.text3,
    )
}

/** The check mark of the selected choice in a picker. Its text says it is selected for TalkBack. */
@Composable
internal fun SelectedMark() {
    TetherIcon(
        icon = TetherIcons.CircleCheck,
        contentDescription = stringResource(R.string.settings_area_selected),
        tint = TetherTheme.colors.accentInk,
        size = TetherSize.iconS,
    )
}

/** The trailing check of a selected picker row, or null for an unselected one. */
internal fun selectedTrailing(selected: Boolean): (@Composable () -> Unit)? =
    if (selected) SelectedTrailing else null

private val SelectedTrailing: @Composable () -> Unit = { SelectedMark() }
