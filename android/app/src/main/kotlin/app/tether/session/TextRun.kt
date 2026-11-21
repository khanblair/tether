package app.tether.session

/** One run of text from a search hit. A [matched] run is the part the query found. */
data class TextRun(
    val text: String,
    val matched: Boolean,
)

/**
 * Splits [text] into runs, marking each case-insensitive occurrence of [query]. A blank query
 * marks nothing. The screen emphasizes matched runs in bold, so the match is not shown by color
 * alone.
 */
fun splitMatches(
    text: String,
    query: String,
): List<TextRun> {
    val needle = query.trim()
    if (needle.isEmpty()) return if (text.isEmpty()) emptyList() else listOf(TextRun(text, false))
    val runs = mutableListOf<TextRun>()
    var cursor = 0
    while (cursor < text.length) {
        val start = text.indexOf(needle, cursor, ignoreCase = true)
        if (start < 0) break
        if (start > cursor) runs += TextRun(text.substring(cursor, start), matched = false)
        val end = start + needle.length
        runs += TextRun(text.substring(start, end), matched = true)
        cursor = end
    }
    if (cursor < text.length) runs += TextRun(text.substring(cursor), matched = false)
    return runs
}
