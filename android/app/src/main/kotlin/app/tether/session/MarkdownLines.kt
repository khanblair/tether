package app.tether.session

/** How one line of prose is drawn. [PLAIN] and [QUOTE] lines are joined into one run each. */
internal enum class LineKind {
    PLAIN,
    HEADING_1,
    HEADING_2,
    HEADING_3,
    BULLET,
    NUMBERED,
    QUOTE,
    RULE,
    TABLE,
}

/**
 * One line of prose. [marker] is the number of a numbered item, such as "3.". [level] is how deep
 * a list item is nested, 0 to [MAX_LEVEL]. A [LineKind.TABLE] line carries its [table] and no text.
 */
internal data class MarkdownLine(
    val kind: LineKind,
    val text: String,
    val marker: String = "",
    val level: Int = 0,
    val table: MarkdownTable? = null,
)

/** The deepest list nesting that is drawn; deeper items sit at this level. */
internal const val MAX_LEVEL = 3

private const val SPACES_PER_LEVEL = 2
private const val TAB_SPACES = 2

private val HEADING = Regex("(#{1,6}) (.*)")
private val BULLET = Regex("(\\s*)[-*+] (.*)")
private val NUMBERED = Regex("(\\s*)(\\d{1,3})[.)] (.*)")
private val QUOTE = Regex("\\s{0,3}> ?(.*)")
private val RULE = Regex("\\s{0,3}([-*_])(\\s*\\1){2,}\\s*")

/**
 * The kind of one line: a heading of level 1 to 6 (levels 3 to 6 share one style), a rule, a
 * quote, a bullet, a numbered item, or plain text. Tables need the next line, so
 * [markdownLines] finds them.
 */
internal fun classifyLine(line: String): MarkdownLine =
    headingLine(line) ?: ruleLine(line) ?: quoteLine(line) ?: bulletLine(line)
        ?: numberedLine(line) ?: MarkdownLine(LineKind.PLAIN, line)

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

private fun ruleLine(line: String): MarkdownLine? =
    if (RULE.matches(line)) MarkdownLine(LineKind.RULE, "") else null

private fun quoteLine(line: String): MarkdownLine? =
    QUOTE.matchEntire(line)?.let { MarkdownLine(LineKind.QUOTE, it.groupValues[1]) }

private fun bulletLine(line: String): MarkdownLine? =
    BULLET.matchEntire(line)?.let {
        MarkdownLine(LineKind.BULLET, it.groupValues[2], level = levelOf(it.groupValues[1]))
    }

private fun numberedLine(line: String): MarkdownLine? =
    NUMBERED.matchEntire(line)?.destructured?.let { (indent, number, text) ->
        MarkdownLine(LineKind.NUMBERED, text, "$number.", levelOf(indent))
    }

/** The nesting level of a list item from its leading whitespace: two spaces or a tab per level. */
private fun levelOf(indent: String): Int =
    (indent.sumOf { if (it == '\t') TAB_SPACES else 1 } / SPACES_PER_LEVEL).coerceAtMost(MAX_LEVEL)

/**
 * The lines of a prose block. Consecutive plain lines merge into one run, so a paragraph wraps as
 * one text, and so do consecutive quote lines. A pipe table becomes one table line. Headings,
 * rules, bullets and numbered items each stay on their own line.
 */
internal fun markdownLines(prose: String): List<MarkdownLine> {
    val source = prose.lines()
    val result = mutableListOf<MarkdownLine>()
    var index = 0
    while (index < source.size) {
        val parsed = parseTableAt(source, index)
        if (parsed != null) {
            result += MarkdownLine(LineKind.TABLE, "", table = parsed.table)
            index += parsed.lineCount
        } else {
            result.appendMerged(classifyLine(source[index]))
            index++
        }
    }
    return result
}

private fun MutableList<MarkdownLine>.appendMerged(line: MarkdownLine) {
    val last = lastOrNull()
    val mergeable = line.kind == LineKind.PLAIN || line.kind == LineKind.QUOTE
    if (mergeable && last?.kind == line.kind) {
        this[lastIndex] = last.copy(text = last.text + "\n" + line.text)
    } else {
        add(line)
    }
}
