package app.tether.session.compose

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace

private val SampleCommands =
    listOf(
        ComposerCommand("compact", "Summarize older context now", null, true, null),
        ComposerCommand("plan", "Switch to Plan mode", null, true, null),
        ComposerCommand("diff", "Open uncommitted changes", null, true, null),
        ComposerCommand("rewind", "Open the checkpoint list", null, true, null),
        ComposerCommand("commit", "Write a message and commit", "[message]", true, null),
        ComposerCommand("review", "Second-model review", null, false, "a second model"),
    )

private val SampleFiles =
    listOf(
        "src/auth/session.py",
        "src/auth/tokens.py",
        "tests/test_login.py",
        "docs/auth/refresh-token-rotation-and-expiry-policy.md",
    )

@Composable
private fun AssistPreviewFrame(content: @Composable () -> Unit) {
    Column(modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4)) { content() }
}

@Preview(name = "Command sheet, full, light", showBackground = true)
@Composable
internal fun CommandSheetFullLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AssistPreviewFrame { CommandSheet(commands = SampleCommands, query = "", onPick = {}) }
    }
}

@Preview(name = "Command sheet, filtered, dark", showBackground = true)
@Composable
internal fun CommandSheetFilteredDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AssistPreviewFrame { CommandSheet(commands = SampleCommands, query = "co", onPick = {}) }
    }
}

@Preview(name = "Command sheet, no match, light", showBackground = true)
@Composable
internal fun CommandSheetNoMatchLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AssistPreviewFrame { CommandSheet(commands = SampleCommands, query = "zz", onPick = {}) }
    }
}

@Preview(name = "Command sheet, loading and failed, dark", showBackground = true)
@Composable
internal fun CommandSheetStatesDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AssistPreviewFrame {
            Column {
                CommandSheet(commands = emptyList(), query = "", onPick = {}, loaded = false)
                CommandSheet(commands = emptyList(), query = "", onPick = {}, failed = true)
            }
        }
    }
}

@Preview(name = "Mention sheet, full, light", showBackground = true)
@Composable
internal fun MentionSheetFullLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AssistPreviewFrame { MentionSheet(files = SampleFiles, onPick = {}) }
    }
}

@Preview(name = "Mention sheet, no match, dark", showBackground = true)
@Composable
internal fun MentionSheetNoMatchDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AssistPreviewFrame { MentionSheet(files = emptyList(), onPick = {}) }
    }
}

@Preview(name = "Mention sheet, loading, light", showBackground = true)
@Composable
internal fun MentionSheetLoadingLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AssistPreviewFrame { MentionSheet(files = emptyList(), onPick = {}, loading = true) }
    }
}

@Preview(name = "Mention chips, dark", showBackground = true)
@Composable
internal fun MentionChipsDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AssistPreviewFrame { MentionChips(paths = SampleFiles.take(2), onRemove = {}) }
    }
}
