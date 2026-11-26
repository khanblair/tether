package app.tether.session

/** How one line of prose is drawn. [PLAIN] lines are joined into one paragraph run. */
internal enum class LineKind { PLAIN, HEADING_1, HEADING_2, HEADING_3, BULLET, NUMBERED }

/** One line of prose. [marker] is the number of a numbered item, such as "3.". */
internal data class MarkdownLine(
    val kind: LineKind,
    val text: String,
    val marker: String = "",
)

private val HEADING = Regex("(#{1,3}) (.*)")
private val BULLET = Regex("\\s*[-*] (.*)")
private val NUMBERED = Regex("\\s*(\\d{1,3})\\. (.*)")

/** The kind of one line: a heading of level 1 to 3, a bullet, a numbered item, or plain text. */
internal fun classifyLine(line: String): MarkdownLine =
    headingLine(line) ?: bulletLine(line) ?: numberedLine(line)
        ?: MarkdownLine(LineKind.PLAIN, line)

private fun headingLine(line: String): MarkdownLine? =
    HEADING.matchEntire(line)?.let { match ->
        val kind =
            when (match.groupValues[1].length) {
                1 -> LineKind.HEADING_1
                2 -> LineKind.HEADING_2
                else -> LineKind.HEADING_3
            }
        MarkdownLine(kind, match.groupValues[2])
    }

private fun bulletLine(line: String): MarkdownLine? =
    BULLET.matchEntire(line)?.let { MarkdownLine(LineKind.BULLET, it.groupValues[1]) }

private fun numberedLine(line: String): MarkdownLine? =
    NUMBERED.matchEntire(line)?.let {
        MarkdownLine(LineKind.NUMBERED, it.groupValues[2], it.groupValues[1] + ".")
    }

/**
 * The lines of a prose block. Consecutive plain lines merge into one run, so a paragraph wraps as
 * one text. Headings, bullets and numbered items each stay on their own line.
 */
internal fun markdownLines(prose: String): List<MarkdownLine> {
    val result = mutableListOf<MarkdownLine>()
    prose.lineSequence().map(::classifyLine).forEach { line ->
        val last = result.lastOrNull()
        if (line.kind == LineKind.PLAIN && last?.kind == LineKind.PLAIN) {
            result[result.lastIndex] = last.copy(text = last.text + "\n" + line.text)
        } else {
            result += line
        }
    }
    return result
}
