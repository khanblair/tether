package app.tether.session

import android.app.Activity
import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import android.speech.RecognizerIntent
import androidx.activity.compose.ManagedActivityResultLauncher
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.TextFieldValue
import app.tether.R
import app.tether.design.tokens.TetherRatio
import app.tether.design.tokens.TetherSpace
import app.tether.session.compose.ComposeAssistDeps
import app.tether.session.compose.ComposeAssistPanel
import app.tether.session.compose.textAtEnd
import app.tether.session.compose.toFieldValue

/**
 * The session input. The field grows with text, capped at 40% of the display height. While
 * [sendBlocked] is true, Send is off: a message cannot go out while an approval waits.
 */
@Composable
fun Composer(
    modelId: String,
    running: Boolean,
    onSend: (String) -> Unit,
    onStop: () -> Unit,
    sendBlocked: Boolean = false,
    assist: ComposeAssistDeps? = null,
) {
    var field by rememberSaveable(stateSaver = TextFieldValue.Saver) {
        mutableStateOf(TextFieldValue(""))
    }
    val input = field.text
    var image by rememberSaveable { mutableStateOf<String?>(null) }
    var speechUnavailable by rememberSaveable { mutableStateOf(false) }
    val picker = rememberImagePicker { image = it }
    val startVoice =
        rememberVoiceInput { words ->
            field = textAtEnd(mergeVoiceDraft(input, words))
        }
    val windowHeight = LocalWindowInfo.current.containerSize.height
    val maximum = with(LocalDensity.current) { windowHeight.toDp() * TetherRatio.composerMax }
    Column(modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4)) {
        assist?.let { deps ->
            ComposeAssistPanel(
                deps = deps,
                text = input,
                cursor = field.selection.end,
                onEdit = { edit -> field = edit.toFieldValue() },
            )
        }
        OutlinedTextField(
            value = field,
            onValueChange = { field = it },
            label = { Text(stringResource(R.string.composer_hint)) },
            modifier = Modifier.fillMaxWidth().heightIn(max = maximum),
            maxLines = Int.MAX_VALUE,
        )
        if (image != null) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.composer_image_staged))
                TextButton(onClick = { image = null }) {
                    Text(stringResource(R.string.composer_image_remove))
                }
            }
        }
        if (speechUnavailable) Text(stringResource(R.string.composer_voice_unavailable))
        ComposerActions(
            modelId = modelId,
            running = running,
            canSend = composerCanSend(input, image, sendBlocked),
            onAttach = { picker.launch("image/*") },
            onVoice = { if (!startVoice()) speechUnavailable = true },
            onStop = onStop,
            onSend = {
                val text = outgoingText(input)
                if (text != null) {
                    onSend(text)
                    field = TextFieldValue("")
                }
            },
        )
    }
}

/**
 * True when Send may go out: the field has text, no image is staged, and nothing blocks the send.
 * A staged image is not sent yet, so it keeps Send off.
 */
internal fun composerCanSend(
    input: String,
    image: String?,
    sendBlocked: Boolean,
): Boolean = input.isNotBlank() && image == null && !sendBlocked

/** The row under the field: attach, model name, voice, and send or stop. */
@Composable
private fun ComposerActions(
    modelId: String,
    running: Boolean,
    canSend: Boolean,
    onAttach: () -> Unit,
    onVoice: () -> Unit,
    onStop: () -> Unit,
    onSend: () -> Unit,
) {
    Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
        TextButton(onClick = onAttach) {
            Text(stringResource(R.string.composer_attachments))
        }
        Text(modelId)
        Button(onClick = onVoice) { Text(stringResource(R.string.composer_voice)) }
        if (running) {
            Button(onClick = onStop) { Text(stringResource(R.string.composer_stop)) }
        } else {
            Button(onClick = onSend, enabled = canSend) {
                Text(stringResource(R.string.composer_send))
            }
        }
    }
}

/** Opens the system picker for one image. [onPicked] gets only image URIs. */
@Composable
private fun rememberImagePicker(
    onPicked: (String) -> Unit,
): ManagedActivityResultLauncher<String, Uri?> {
    val context = LocalContext.current
    return rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        if (uri != null) {
            val type = context.contentResolver.getType(uri).orEmpty()
            if (isImageType(type)) onPicked(uri.toString())
        }
    }
}

/**
 * Starts the speech recognizer and adds its first result through [onWords]. The returned function
 * gives false when the device has no recognizer, so the caller can say voice input is unavailable.
 */
@Composable
private fun rememberVoiceInput(onWords: (String) -> Unit): () -> Boolean {
    val speech =
        rememberLauncherForActivityResult(
            ActivityResultContracts.StartActivityForResult(),
        ) { result ->
            if (result.resultCode == Activity.RESULT_OK) {
                result.data
                    ?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)
                    ?.firstOrNull()
                    ?.let(onWords)
            }
        }
    return {
        try {
            speech.launch(speechIntent())
            true
        } catch (_: ActivityNotFoundException) {
            false
        }
    }
}

private fun speechIntent(): Intent =
    Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
        putExtra(
            RecognizerIntent.EXTRA_LANGUAGE_MODEL,
            RecognizerIntent.LANGUAGE_MODEL_FREE_FORM,
        )
    }

/** Blank input never starts a turn. Preserve line breaks inside a message. */
internal fun outgoingText(input: String): String? = input.trim().takeIf(String::isNotEmpty)

/** Adds recognized speech to the draft without erasing text already typed. */
internal fun mergeVoiceDraft(
    draft: String,
    speech: String,
): String = listOf(draft.trimEnd(), speech.trim()).filter(String::isNotEmpty).joinToString(" ")
