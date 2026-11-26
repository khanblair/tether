package app.tether.session

import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle

/**
 * Styles inline Markdown in one line of prose. Code spans come first, so their text is never
 * styled. A link shows only its text. Delimiters that do not close stay as plain characters, so
 * unfinished syntax while a reply streams never throws and never disappears.
 */
internal fun inlineMarkdown(
    source: String,
    styles: InlineStyles = InlineStyles(),
): AnnotatedString = buildAnnotatedString { appendInline(source, styles) }

private fun AnnotatedString.Builder.appendInline(
    text: String,
    styles: InlineStyles,
) {
    var i = 0
    while (i < text.length) {
        val used =
            when {
                text[i] == '`' -> appendCode(text, i, styles)
                text.startsWith("**", i) -> appendEmphasis(text, i, "**", styles.bold, styles)
                text[i] == '*' -> appendEmphasis(text, i, "*", styles.italic, styles)
                text[i] == '[' -> appendLink(text, i, styles)
                else -> 0
            }
        if (used == 0) {
            append(text[i])
            i++
        } else {
            i += used
        }
    }
}

/** A code span from the backtick at [start]. Returns the characters used, or 0 when unclosed. */
private fun AnnotatedString.Builder.appendCode(
    text: String,
    start: Int,
    styles: InlineStyles,
): Int {
    val end = text.indexOf('`', start + 1)
    if (end <= start + 1) return 0
    withStyle(styles.code) { append(text.substring(start + 1, end)) }
    return end - start + 1
}

/**
 * Bold (`**`) or italic (`*`) starting at [start]. The opener must touch text, and the closer
 * must follow text, so "2 * 3 * 4" stays plain. Returns the characters used, or 0.
 */
private fun AnnotatedString.Builder.appendEmphasis(
    text: String,
    start: Int,
    delimiter: String,
    style: SpanStyle,
    styles: InlineStyles,
): Int {
    val contentStart = start + delimiter.length
    val opens = contentStart < text.length && !text[contentStart].isWhitespace()
    val end = if (opens) closerAt(text, contentStart, delimiter) else -1
    if (end < 0) return 0
    withStyle(style) { appendInline(text.substring(contentStart, end), styles) }
    return end + delimiter.length - start
}

/** The index of the delimiter that closes an emphasis span, or -1 when none does. */
private fun closerAt(
    text: String,
    contentStart: Int,
    delimiter: String,
): Int {
    var end = text.indexOf(delimiter, contentStart)
    while (end > contentStart) {
        val partOfDouble = delimiter.length == 1 && text.getOrNull(end + 1) == '*'
        if (!text[end - 1].isWhitespace() && !partOfDouble) return end
        end = text.indexOf(delimiter, end + 1)
    }
    return -1
}

/** A link `[text](url)`: only the text is shown, underlined and not clickable. */
private fun AnnotatedString.Builder.appendLink(
    text: String,
    start: Int,
    styles: InlineStyles,
): Int {
    val close = linkClose(text, start)
    if (close < 0) return 0
    val labelEnd = text.indexOf(']', start + 1)
    withStyle(styles.link) { append(text.substring(start + 1, labelEnd)) }
    return close - start + 1
}

/** The index of the ")" that closes the link at [start], or -1 when it is not a link. */
private fun linkClose(
    text: String,
    start: Int,
): Int {
    val labelEnd = text.indexOf(']', start + 1)
    if (labelEnd <= start + 1 || !text.startsWith("](", labelEnd)) return -1
    val close = text.indexOf(')', labelEnd + 2)
    val url = if (close < 0) "" else text.substring(labelEnd + 2, close)
    return if (url.isEmpty() || url.any(Char::isWhitespace)) -1 else close
}
