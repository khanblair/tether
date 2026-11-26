package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Density
import app.tether.R
import app.tether.core.generated.MessageView
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSpace

private const val EDIT_MIN_LINES = 3
private const val EDIT_MAX_LINES = 10
private const val PREVIEW_LARGE_FONT = 2f
private const val PREVIEW_TEXT = "Fix the failing login test and add a regression test"

/**
 * The sheet that edits a user message. Its draft starts as the message text, with the cursor at
 * the end. [onSend] gets the edited text; the caller decides whether it was sent.
 */
@Composable
fun EditMessageSheet(
    message: MessageView,
    onSend: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.chat_edit_message), onDismiss = onDismiss) {
        EditMessageBody(
            original = plainText(message),
            draftKey = message.id,
            onSend = onSend,
        )
    }
}

/**
 * The field, the helper line, and the Send button of the edit sheet. The draft is saved, so a
 * rotation keeps what was typed. Send is off for blank text and for text that did not change.
 */
@Composable
internal fun EditMessageBody(
    original: String,
    draftKey: String,
    onSend: (String) -> Unit,
) {
    var draft by rememberSaveable(draftKey, stateSaver = TextFieldValue.Saver) {
        mutableStateOf(TextFieldValue(original, TextRange(original.length)))
    }
    val text = draft.text
    val canSend = text.isNotBlank() && text.trim() != original.trim()
    Column(
        modifier =
            Modifier
                .verticalScroll(rememberScrollState())
                .padding(horizontal = TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        OutlinedTextField(
            value = draft,
            onValueChange = { draft = it },
            label = { Text(stringResource(R.string.chat_edit_field)) },
            minLines = EDIT_MIN_LINES,
            maxLines = EDIT_MAX_LINES,
            modifier = Modifier.fillMaxWidth(),
        )
        Text(
            text = stringResource(R.string.chat_edit_helper),
            style = MaterialTheme.typography.labelMedium,
            color = TetherTheme.colors.text2,
        )
        PrimaryButton(
            label = stringResource(R.string.chat_edit_send),
            onClick = { onSend(text) },
            modifier = Modifier.fillMaxWidth(),
            enabled = canSend,
        )
    }
}

@Preview(name = "Edit body, light", showBackground = true)
@Composable
internal fun EditMessagePreview() = EditPreview(fontScale = 1f)

@Preview(name = "Edit body, light, 200% font", showBackground = true)
@Composable
internal fun EditMessageLargeFontPreview() = EditPreview(fontScale = PREVIEW_LARGE_FONT)

@Composable
private fun EditPreview(fontScale: Float) {
    val density = LocalDensity.current
    TetherTheme(mode = ThemeMode.Light) {
        CompositionLocalProvider(LocalDensity provides Density(density.density, fontScale)) {
            EditMessageBody(original = PREVIEW_TEXT, draftKey = "preview", onSend = {})
        }
    }
}
