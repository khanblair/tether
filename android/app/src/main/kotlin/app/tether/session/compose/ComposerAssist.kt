package app.tether.session.compose

import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue

/** What the composer's text asks for at the cursor: nothing, the commands, or the file picker. */
sealed interface AssistMode {
    /** No sheet is open. */
    data object None : AssistMode

    /** The text is a slash command being typed. [prefix] is the text after the slash. */
    data class Command(
        val prefix: String,
    ) : AssistMode

    /** The cursor is inside an `@` mention. [start] is the `@`, and [end] is where it stops. */
    data class Mention(
        val query: String,
        val start: Int,
        val end: Int,
    ) : AssistMode
}

/**
 * One `@path` in the text. [start] is the `@`. [end] is just after the path or its closing quote.
 */
data class MentionToken(
    val path: String,
    val start: Int,
    val end: Int,
)

/** The text and cursor after an edit. */
data class ComposerEdit(
    val text: String,
    val cursor: Int,
)

/**
 * Reads what the text asks for at [cursor]. A command is a slash at the very start with no space
 * before the cursor. A mention is an `@` at the start of a word, with no space between it and the
 * cursor. An `@` inside a word, such as in an email address, is not a mention.
 */
fun assist(
    text: String,
    cursor: Int,
): AssistMode {
    val at = cursor.coerceIn(0, text.length)
    return commandAt(text, at)?.let { AssistMode.Command(it) }
        ?: mentionAt(text, at)
        ?: AssistMode.None
}

/** Filters [commands] to the ones whose name starts with [prefix], ignoring case. */
fun commandsMatching(
    commands: List<ComposerCommand>,
    prefix: String,
): List<ComposerCommand> = commands.filter { it.name.startsWith(prefix, ignoreCase = true) }

/**
 * Replaces the typed command with `/name`. A command that takes arguments gets a trailing space,
 * and the rest of the text follows it. Text that does not start with a slash comes back unchanged.
 */
fun applyCommand(
    text: String,
    name: String,
    takesArguments: Boolean,
): ComposerEdit {
    if (!text.startsWith("/")) return ComposerEdit(text, text.length)
    val wordEnd = text.indexOfFirst(Char::isWhitespace).let { if (it < 0) text.length else it }
    val rest = text.substring(wordEnd)
    val head = "/$name"
    return if (takesArguments) {
        val withSpace = "$head "
        ComposerEdit(withSpace + rest.trimStart(), withSpace.length)
    } else {
        ComposerEdit(head + rest, head.length)
    }
}

/** Shortens [text] to [max] characters by cutting the middle and keeping both ends. */
fun truncateMiddle(
    text: String,
    max: Int = MIDDLE_TRUNCATE_MAX,
): String {
    if (text.length <= max) return text
    val keep = (max - 1).coerceAtLeast(2)
    val head = (keep + 1) / 2
    val tail = keep / 2
    return text.take(head) + "…" + text.takeLast(tail)
}

/** The edit for a composer field: the text, with the cursor placed at [ComposerEdit.cursor]. */
internal fun ComposerEdit.toFieldValue(): TextFieldValue = TextFieldValue(text, TextRange(cursor))

/** A field value with [text] and the cursor at its end. */
internal fun textAtEnd(text: String): TextFieldValue = TextFieldValue(text, TextRange(text.length))

/** Longest path shown in full before [truncateMiddle] shortens it. */
internal const val MIDDLE_TRUNCATE_MAX = 40

private fun commandAt(
    text: String,
    cursor: Int,
): String? {
    val head = text.substring(0, cursor)
    val isCommand = cursor >= 1 && text[0] == '/' && head.none(Char::isWhitespace)
    return if (isCommand) head.substring(1) else null
}
