package app.tether.session

import androidx.annotation.StringRes
import app.tether.R
import app.tether.design.components.ChipTone

/** The rows of the plus sheet, in the order the prototype lists them. */
internal enum class PlusRow { PHOTO, CAMERA, FILE, MENTION, COMMANDS }

/** Why a plus row cannot be used. Each reason has its own line of text. */
internal enum class PlusBlock(
    @param:StringRes val labelRes: Int,
) {
    NO_VISION(R.string.compose_plus_block_vision),
    NOT_AVAILABLE(R.string.compose_plus_block_file),
    DRAFT_NOT_EMPTY(R.string.compose_plus_block_draft),
}

/** A draft after an edit, with the cursor position to show. */
internal data class DraftEdit(
    val text: String,
    val cursor: Int,
)

/** The mode chip is filled accent for Auto, red for Bypass, and neutral for the other modes. */
internal fun modeChipTone(mode: SessionMode): ChipTone =
    when (mode) {
        SessionMode.AUTO -> ChipTone.Accent
        SessionMode.BYPASS -> ChipTone.Danger
        SessionMode.PLAN, SessionMode.ASK, SessionMode.ACCEPT_EDITS -> ChipTone.Neutral
    }

/** Why [row] is off, or null when the row can be used. */
internal fun plusRowBlock(
    row: PlusRow,
    supportsImages: Boolean,
    draft: String,
): PlusBlock? =
    when (row) {
        PlusRow.PHOTO, PlusRow.CAMERA -> if (supportsImages) null else PlusBlock.NO_VISION
        PlusRow.FILE -> PlusBlock.NOT_AVAILABLE
        PlusRow.MENTION -> null
        PlusRow.COMMANDS -> if (draft.isEmpty()) null else PlusBlock.DRAFT_NOT_EMPTY
    }

/**
 * Puts an "@" over the selection [start, end). A space goes first when the text before the
 * cursor is a word, so the mention never runs into it.
 */
internal fun mentionEdit(
    text: String,
    start: Int,
    end: Int,
): DraftEdit {
    val from = start.coerceIn(0, text.length)
    val to = end.coerceIn(from, text.length)
    val before = text.substring(0, from)
    val lead = if (before.isNotEmpty() && !before.last().isWhitespace()) " " else ""
    val inserted = "$lead@"
    return DraftEdit(
        text = before + inserted + text.substring(to),
        cursor = before.length + inserted.length,
    )
}

/** "/" goes in only at the start of an empty draft. Null when the draft has text. */
internal fun commandEdit(text: String): DraftEdit? =
    if (text.isEmpty()) DraftEdit(text = "/", cursor = 1) else null

/** The name of a camera photo in the cache folder. It is unique per capture time. */
internal fun cameraFileName(epochMillis: Long): String = "photo-$epochMillis.jpg"
