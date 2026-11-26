package app.tether.session

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace

private const val SAMPLE_MODEL = "Sonnet 5.5"
private const val SAMPLE_DRAFT = "Fix the failing login test and add a regression test"
private const val SAMPLE_LINES = 6

@Composable
private fun ComposerPreview(
    text: String,
    mode: SessionMode = SessionMode.ACCEPT_EDITS,
    running: Boolean = false,
    canSend: Boolean = text.isNotBlank(),
    staged: Boolean = false,
    maxHeight: Dp = 240.dp,
) {
    TetherTheme(mode = ThemeMode.Light) {
        Column(modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4)) {
            ComposerFrame {
                if (staged) StagedImageRow(onRemove = {})
                ComposerField(
                    value = TextFieldValue(text, TextRange(text.length)),
                    onValueChange = {},
                    maxHeight = maxHeight,
                )
                ComposerBar(
                    mode = mode,
                    modelLabel = SAMPLE_MODEL,
                    running = running,
                    canSend = canSend,
                    onPlus = {},
                    onOpenMode = {},
                    onOpenModel = {},
                    onVoice = {},
                    onStop = {},
                    onSend = {},
                )
            }
        }
    }
}

@Preview(name = "Empty, light", showBackground = true)
@Composable
internal fun ComposerEmptyPreview() {
    ComposerPreview(text = "")
}

@Preview(name = "Typing, light", showBackground = true)
@Composable
internal fun ComposerTypingPreview() {
    ComposerPreview(text = "Fix the login test")
}

@Preview(name = "Multi-line at the height cap, light", showBackground = true)
@Composable
internal fun ComposerMultiLinePreview() {
    ComposerPreview(
        text = List(SAMPLE_LINES) { "Line ${it + 1} of a long message" }.joinToString("\n"),
        maxHeight = 120.dp,
    )
}

@Preview(name = "Running, Stop, light", showBackground = true)
@Composable
internal fun ComposerRunningPreview() {
    ComposerPreview(text = "", running = true)
}

@Preview(name = "Auto chip, light", showBackground = true)
@Composable
internal fun ComposerAutoPreview() {
    ComposerPreview(text = SAMPLE_DRAFT, mode = SessionMode.AUTO)
}

@Preview(name = "Bypass chip, light", showBackground = true)
@Composable
internal fun ComposerBypassPreview() {
    ComposerPreview(text = SAMPLE_DRAFT, mode = SessionMode.BYPASS)
}

@Preview(name = "Staged image, light", showBackground = true)
@Composable
internal fun ComposerStagedImagePreview() {
    ComposerPreview(text = SAMPLE_DRAFT, staged = true, canSend = false)
}

@Preview(name = "Send disabled, light", showBackground = true)
@Composable
internal fun ComposerSendDisabledPreview() {
    ComposerPreview(text = "", canSend = false)
}

@Preview(name = "Plus sheet, light", showBackground = true)
@Composable
internal fun PlusSheetPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        PlusSheet(
            draft = "",
            supportsImages = false,
            onDismiss = {},
            onPick = {},
        )
    }
}
