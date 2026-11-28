package app.tether.session

import android.net.Uri
import androidx.activity.compose.ManagedActivityResultLauncher
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.Dp
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherRatio
import app.tether.design.tokens.TetherSpace
import app.tether.session.compose.ComposeAssistDeps
import app.tether.session.compose.ComposeAssistPanel
import app.tether.session.compose.textAtEnd
import app.tether.session.compose.toFieldValue
import app.tether.session.voice.rememberVoiceSheet
import java.io.File

/**
 * The session input. The field grows with text, capped at 40% of the display height. While
 * [sendBlocked] is true, Send is off: a message cannot go out while an approval waits. The plus
 * sheet holds photo, file, mention and command entries. [supportsImages] turns Photo on.
 */
@Composable
fun Composer(
    modelLabel: String,
    mode: SessionMode,
    running: Boolean,
    onSend: (String) -> Unit,
    onStop: () -> Unit,
    onOpenModel: (() -> Unit)? = null,
    onOpenMode: (() -> Unit)? = null,
    supportsImages: Boolean = false,
    sendBlocked: Boolean = false,
    assist: ComposeAssistDeps? = null,
    initialDraft: String? = null,
) {
    var field by rememberSaveable(stateSaver = TextFieldValue.Saver) {
        val draft = initialDraft.orEmpty()
        mutableStateOf(TextFieldValue(draft, TextRange(draft.length)))
    }
    val input = field.text
    var image by rememberSaveable { mutableStateOf<String?>(null) }
    var stagedCamera by rememberSaveable { mutableStateOf<String?>(null) }
    var cameraUnavailable by rememberSaveable { mutableStateOf(false) }
    var plusOpen by rememberSaveable { mutableStateOf(false) }
    val picker =
        rememberImagePicker {
            discardCamera(stagedCamera)
            stagedCamera = null
            image = it
        }
    val takePhoto =
        rememberCameraCapture(
            onCaptured = { file ->
                stagedCamera = file.path
                image = Uri.fromFile(file).toString()
            },
            onUnavailable = { cameraUnavailable = true },
        )
    val openVoice =
        rememberVoiceSheet(onInsert = { words ->
            field = textAtEnd(mergeVoiceDraft(field.text, words))
        })
    val windowHeight = LocalWindowInfo.current.containerSize.height
    val maximum = with(LocalDensity.current) { windowHeight.toDp() * TetherRatio.composerMax }
    val onPick: (PlusRow) -> Unit = { row ->
        plusOpen = false
        when (row) {
            PlusRow.PHOTO -> picker.launch("image/*")
            PlusRow.CAMERA -> takePhoto()
            PlusRow.FILE -> Unit
            PlusRow.MENTION, PlusRow.COMMANDS -> field = insertPlusText(row, field)
        }
    }
    val removeStaged: () -> Unit = {
        discardCamera(stagedCamera)
        stagedCamera = null
        image = null
    }
    Column(modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space3)) {
        assist?.let { deps ->
            ComposeAssistPanel(
                deps = deps,
                text = input,
                cursor = field.selection.end,
                onEdit = { edit -> field = edit.toFieldValue() },
            )
        }
        ComposerFrame {
            StagedImageSlot(staged = image != null, onRemove = removeStaged)
            ComposerField(
                value = field,
                onValueChange = { field = it },
                maxHeight = maximum,
            )
            ComposerBar(
                mode = mode,
                modelLabel = modelLabel,
                running = running,
                canSend = composerCanSend(input, image, sendBlocked),
                onPlus = { plusOpen = true },
                onOpenMode = onOpenMode,
                onOpenModel = onOpenModel,
                onVoice = openVoice,
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
        ComposerNotices(cameraUnavailable = cameraUnavailable)
    }
    if (plusOpen) {
        PlusSheet(
            draft = input,
            supportsImages = supportsImages,
            onDismiss = { plusOpen = false },
            onPick = onPick,
        )
    }
}

/** The rounded input box: a surface with a 1 dp input line border. */
@Composable
internal fun ComposerFrame(content: @Composable ColumnScope.() -> Unit) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(colors.surface)
                .border(TetherBorder.hairline, colors.lineInput, shape)
                .padding(TetherSpace.space1),
        content = content,
    )
}

/** The text field with an inline placeholder, not a floating label. It grows up to [maxHeight]. */
@Composable
internal fun ComposerField(
    value: TextFieldValue,
    onValueChange: (TextFieldValue) -> Unit,
    maxHeight: Dp,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(start = TetherSpace.space2, end = TetherSpace.space2)
                .padding(top = TetherSpace.space2, bottom = TetherSpace.space1),
    ) {
        BasicTextField(
            value = value,
            onValueChange = onValueChange,
            modifier = Modifier.fillMaxWidth().heightIn(max = maxHeight),
            textStyle = MaterialTheme.typography.bodyMedium.copy(color = colors.text),
            cursorBrush = SolidColor(colors.accent),
            maxLines = Int.MAX_VALUE,
            decorationBox = { inner ->
                Box {
                    if (value.text.isEmpty()) {
                        Text(
                            text = stringResource(R.string.composer_hint),
                            style = MaterialTheme.typography.bodyMedium,
                            color = colors.text3,
                        )
                    }
                    inner()
                }
            },
        )
    }
}

/** A short line under the composer when the camera cannot be used. */
@Composable
private fun ComposerNotices(cameraUnavailable: Boolean) {
    val modifier = Modifier.padding(horizontal = TetherSpace.space3)
    if (cameraUnavailable) {
        Text(stringResource(R.string.compose_camera_unavailable), modifier = modifier)
    }
}

/** Shows the staged row only while an image is picked. */
@Composable
private fun StagedImageSlot(
    staged: Boolean,
    onRemove: () -> Unit,
) {
    if (staged) StagedImageRow(onRemove = onRemove)
}

/** Says an image is picked and offers to remove it. Sending images is not available yet. */
@Composable
internal fun StagedImageRow(onRemove: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(stringResource(R.string.composer_image_staged))
        TextButton(onClick = onRemove) {
            Text(stringResource(R.string.composer_image_remove))
        }
    }
}

private fun DraftEdit.toFieldValue(): TextFieldValue = TextFieldValue(text, TextRange(cursor))

/** The field after a mention or a command goes in. Other rows leave the field as it is. */
private fun insertPlusText(
    row: PlusRow,
    value: TextFieldValue,
): TextFieldValue =
    when (row) {
        PlusRow.MENTION -> {
            val sel = value.selection
            mentionEdit(value.text, sel.min, sel.max).toFieldValue()
        }
        PlusRow.COMMANDS -> commandEdit(value.text)?.toFieldValue() ?: value
        PlusRow.PHOTO, PlusRow.CAMERA, PlusRow.FILE -> value
    }

/** Deletes a camera photo that was staged but is no longer wanted. */
private fun discardCamera(path: String?) {
    path?.let { File(it).delete() }
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

/** Blank input never starts a turn. Preserve line breaks inside a message. */
internal fun outgoingText(input: String): String? = input.trim().takeIf(String::isNotEmpty)

/** Adds recognized speech to the draft without erasing text already typed. */
internal fun mergeVoiceDraft(
    draft: String,
    speech: String,
): String = listOf(draft.trimEnd(), speech.trim()).filter(String::isNotEmpty).joinToString(" ")
