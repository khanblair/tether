package app.tether.linux

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

@Preview(name = "Linux shell, light", showBackground = true)
@Composable
internal fun LinuxShellLightPreview() {
    TetherTheme { PreviewTerminal() }
}

@Preview(name = "Linux shell, dark", showBackground = true)
@Composable
internal fun LinuxShellDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { PreviewTerminal() }
}

@Preview(name = "Linux shell, 200 percent text", fontScale = 2f, showBackground = true)
@Composable
internal fun LinuxShellLargeTextPreview() {
    TetherTheme { PreviewTerminal() }
}

@Preview(name = "Linux shell, 320 dp", widthDp = 320, showBackground = true)
@Composable
internal fun LinuxShellNarrowPreview() {
    TetherTheme { PreviewTerminal() }
}

@Composable
private fun PreviewTerminal() {
    val buffer = TerminalBuffer(rows = 8, columns = 36)
    val parser = AnsiParser(buffer)
    parser.feed("$ [32mprintf[0m 'Linux ready'\n".toByteArray())
    parser.feed(
        "[31mred[0m [38;5;45mcyan[0m [1;4mbold underline[0m".toByteArray(),
    )
    ShellTerminal(
        frame = buffer.frame(),
        onInput = {},
        onResize = { _, _ -> },
        onCopy = {},
    )
}
