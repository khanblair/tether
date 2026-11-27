package app.tether.settings.executors

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherSheet
import app.tether.linux.LinuxBackgroundMode
import app.tether.settings.selectedTrailing

/** The Linux background setting row in the Executors screen. */
@Composable
internal fun LinuxBackgroundSettingRow(
    mode: LinuxBackgroundMode,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    ListRow(
        title = stringResource(R.string.linux_background_title),
        subtitle = stringResource(mode.rowTitleRes()),
        onClick = onClick,
        modifier = modifier,
    )
}

/** The two choices for the Linux background service. */
@Composable
internal fun LinuxBackgroundPickSheet(
    current: LinuxBackgroundMode,
    onPick: (LinuxBackgroundMode) -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(
        title = stringResource(R.string.linux_background_sheet_title),
        onDismiss = onDismiss,
    ) {
        ListGroup {
            LinuxBackgroundMode.entries.forEach { mode ->
                ListRow(
                    title = stringResource(mode.titleRes()),
                    subtitle = stringResource(mode.descriptionRes()),
                    trailing = selectedTrailing(mode == current),
                    onClick = { onPick(mode) },
                )
            }
        }
    }
}

@Preview(name = "Linux background row, light", showBackground = true)
@Composable
internal fun LinuxBackgroundRowLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        LinuxBackgroundSettingRow(mode = LinuxBackgroundMode.WhileRunning, onClick = {})
    }
}

@Preview(name = "Linux background row, dark", showBackground = true)
@Composable
internal fun LinuxBackgroundRowDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        LinuxBackgroundSettingRow(mode = LinuxBackgroundMode.WhenInstalled, onClick = {})
    }
}

@Preview(name = "Linux background row, 200% font, 320 dp", widthDp = 320, fontScale = 2f)
@Composable
internal fun LinuxBackgroundRowNarrowPreview() {
    TetherTheme { LinuxBackgroundSettingRow(mode = LinuxBackgroundMode.WhileRunning, onClick = {}) }
}

@Preview(name = "Linux background picker, light", showBackground = true)
@Composable
internal fun LinuxBackgroundPickerLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        LinuxBackgroundPickSheet(
            current = LinuxBackgroundMode.WhileRunning,
            onPick = {},
            onDismiss = {},
        )
    }
}

@Preview(name = "Linux background picker, dark", showBackground = true)
@Composable
internal fun LinuxBackgroundPickerDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        LinuxBackgroundPickSheet(
            current = LinuxBackgroundMode.WhenInstalled,
            onPick = {},
            onDismiss = {},
        )
    }
}

@Preview(name = "Linux background picker, 200% font, 320 dp", widthDp = 320, fontScale = 2f)
@Composable
internal fun LinuxBackgroundPickerNarrowPreview() {
    TetherTheme {
        LinuxBackgroundPickSheet(
            current = LinuxBackgroundMode.WhileRunning,
            onPick = {},
            onDismiss = {},
        )
    }
}

private fun LinuxBackgroundMode.rowTitleRes(): Int =
    when (this) {
        LinuxBackgroundMode.WhileRunning -> R.string.linux_background_row_while_running
        LinuxBackgroundMode.WhenInstalled -> R.string.linux_background_row_when_installed
    }

private fun LinuxBackgroundMode.titleRes(): Int =
    when (this) {
        LinuxBackgroundMode.WhileRunning -> R.string.linux_background_while_running_title
        LinuxBackgroundMode.WhenInstalled -> R.string.linux_background_when_installed_title
    }

private fun LinuxBackgroundMode.descriptionRes(): Int =
    when (this) {
        LinuxBackgroundMode.WhileRunning -> R.string.linux_background_while_running_description
        LinuxBackgroundMode.WhenInstalled -> R.string.linux_background_when_installed_description
    }
