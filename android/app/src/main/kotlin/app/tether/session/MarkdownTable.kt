package app.tether.session

/** How the text of one table column is aligned, from the colons of the delimiter row. */
internal enum class ColumnAlign { START, CENTER, END }

/** A pipe table: a header row, the column alignments, and body rows with one cell per column. */
internal data class MarkdownTable(
    val header: List<String>,
    val align: List<ColumnAlign>,
    val rows: List<List<String>>,
)

/** A parsed table and the number of source lines it used. */
internal data class TableParse(
    val table: MarkdownTable,
    val lineCount: Int,
)

private val DELIMITER_CELL = Regex(":?-{3,}:?")

/**
 * The table that starts at [start] in [lines], or null. A table is a header row with at least one
 * pipe, then a delimiter row such as `|---|:--:|` with the same number of cells, then body rows
 * while the lines keep a pipe. A header with no delimiter row yet stays plain text, so a reply
 * that is still streaming shows its rows as text until the delimiter arrives.
 */
internal fun parseTableAt(
    lines: List<String>,
    start: Int,
): TableParse? {
    val header = lines.getOrNull(start)?.takeIf { '|' in it }?.let(::splitTableRow)
    val delimiter = lines.getOrNull(start + 1)?.takeIf { '|' in it }?.let(::splitTableRow)
    val valid = header != null && delimiter != null && delimiter.size == header.size
    return if (valid && delimiter.all { DELIMITER_CELL.matches(it) }) {
        val body =
            lines
                .drop(start + 2)
                .takeWhile { it.isNotBlank() && '|' in it }
                .map { splitTableRow(it).fitTo(header.size) }
        TableParse(
            MarkdownTable(header, delimiter.map(::columnAlign), body),
            lineCount = 2 + body.size,
        )
    } else {
        null
    }
}

private fun columnAlign(cell: String): ColumnAlign =
    when {
        cell.startsWith(':') && cell.endsWith(':') -> ColumnAlign.CENTER
        cell.endsWith(':') -> ColumnAlign.END
        else -> ColumnAlign.START
    }

/** Pads or cuts [this] to [size] cells, so a short streaming row still has every column. */
private fun List<String>.fitTo(size: Int): List<String> =
    List(size) { index -> getOrElse(index) { "" } }

/**
 * The cells of one table row. One leading and one trailing pipe are dropped, `\|` stays a pipe
 * inside a cell, and each cell is trimmed.
 */
internal fun splitTableRow(line: String): List<String> {
    val cells = mutableListOf<String>()
    val cell = StringBuilder()
    var i = 0
    while (i < line.length) {
        val c = line[i]
        if (c == '\\' && line.getOrNull(i + 1) == '|') {
            cell.append('|')
            i += 2
        } else {
            if (c == '|') {
                cells += cell.toString()
                cell.clear()
            } else {
                cell.append(c)
            }
            i++
        }
    }
    cells += cell.toString()
    val trimmed = cells.map { it.trim() }
    val withoutStart = if (line.trimStart().startsWith('|')) trimmed.drop(1) else trimmed
    return if (line.trimEnd().endsWith('|') && !line.trimEnd().endsWith("\\|")) {
        withoutStart.dropLast(1)
    } else {
        withoutStart
    }
}
