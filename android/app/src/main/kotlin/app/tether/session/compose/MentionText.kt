package app.tether.session.compose

/**
 * Every `@path` in [text], in order. A quoted path may hold spaces. An unclosed quote is not a
 * mention.
 */
fun mentionsIn(text: String): List<MentionToken> {
    val found = mutableListOf<MentionToken>()
    var index = 0
    while (index < text.length) {
        val token =
            if (text[index] == '@' &&
                atWordStart(text, index)
            ) {
                readMention(text, index)
            } else {
                null
            }
        if (token != null) {
            found += token
            index = token.end
        } else {
            index++
        }
    }
    return found
}

/**
 * Replaces [mention] with `@path`, followed by a space unless the text already has one there. A
 * path with spaces is written in quotes. The cursor goes after the inserted text.
 */
fun applyMention(
    text: String,
    mention: AssistMode.Mention,
    path: String,
): ComposerEdit {
    val after = text.substring(mention.end)
    val separator = if (after.firstOrNull()?.isWhitespace() == true) "" else " "
    val inserted = "@${quotePath(path)}$separator"
    val newText = text.substring(0, mention.start) + inserted + after
    return ComposerEdit(newText, mention.start + inserted.length)
}

/**
 * Deletes every mention of [path] from [text], with one neighboring space so no double space is
 * left behind. The cursor goes to where the first removed mention started.
 */
fun removeMention(
    text: String,
    path: String,
): ComposerEdit {
    var result = text
    var cursor = text.length
    mentionsIn(text).filter { it.path == path }.asReversed().forEach { token ->
        val (from, to) = spanToRemove(result, token)
        result = result.removeRange(from, to)
        cursor = from
    }
    return ComposerEdit(result, cursor)
}

internal fun mentionAt(
    text: String,
    cursor: Int,
): AssistMode.Mention? = quotedMentionAt(text, cursor) ?: plainMentionAt(text, cursor)

/** A quoted mention, `@"path with spaces`, where the cursor is after the opening quote. */
private fun quotedMentionAt(
    text: String,
    cursor: Int,
): AssistMode.Mention? {
    val quote = text.lastIndexOf('"', cursor - 1)
    if (quote < 1 || text[quote - 1] != '@' || !atWordStart(text, quote - 1)) return null
    val closing = text.indexOf('"', cursor)
    val end = if (closing >= 0) closing + 1 else cursor
    return AssistMode.Mention(
        query = text.substring(quote + 1, cursor),
        start = quote - 1,
        end = end,
    )
}

/** A plain mention, `@path`, where the cursor is inside the word that starts with the `@`. */
private fun plainMentionAt(
    text: String,
    cursor: Int,
): AssistMode.Mention? {
    val at = openAtBefore(text, cursor)
    if (at < 0 || !atWordStart(text, at)) return null
    return AssistMode.Mention(
        query = text.substring(at + 1, cursor),
        start = at,
        end = wordEnd(text, cursor),
    )
}

/** The index of the `@` in the word before [cursor], or -1 when that word has no `@`. */
private fun openAtBefore(
    text: String,
    cursor: Int,
): Int {
    var index = cursor - 1
    while (index >= 0 && !text[index].isWhitespace() && text[index] != '@') index--
    return if (index >= 0 && text[index] == '@') index else -1
}

private fun readMention(
    text: String,
    at: Int,
): MentionToken? {
    val start = at + 1
    if (start < text.length && text[start] == '"') return readQuoted(text, at)
    return readWord(text, at)
}

/** A quoted path, `@"like this"`. Null when the closing quote is missing or the path is empty. */
private fun readQuoted(
    text: String,
    at: Int,
): MentionToken? {
    val open = at + 1
    val closing = text.indexOf('"', open + 1)
    if (closing <= open + 1) return null
    return MentionToken(text.substring(open + 1, closing), at, closing + 1)
}

/** A plain path, `@word`. Null when nothing follows the `@`. */
private fun readWord(
    text: String,
    at: Int,
): MentionToken? {
    val start = at + 1
    val end = wordEnd(text, start)
    if (end == start) return null
    return MentionToken(text.substring(start, end), at, end)
}

private fun spanToRemove(
    text: String,
    token: MentionToken,
): Pair<Int, Int> =
    when {
        token.end < text.length && text[token.end] == ' ' -> token.start to token.end + 1
        token.start > 0 && text[token.start - 1] == ' ' -> token.start - 1 to token.end
        else -> token.start to token.end
    }

private fun quotePath(path: String): String =
    if (path.any(Char::isWhitespace)) "\"$path\"" else path

/** True when the character at [at] starts a word: the start of the text or after whitespace. */
private fun atWordStart(
    text: String,
    at: Int,
): Boolean = at == 0 || text[at - 1].isWhitespace()

/** The index just past the run of non-whitespace characters that starts at [from]. */
private fun wordEnd(
    text: String,
    from: Int,
): Int {
    var index = from
    while (index < text.length && !text[index].isWhitespace()) index++
    return index
}
