package app.tether.linux

/** A terminal colour: either an ANSI palette entry or a packed truecolour value. */
data class TerminalColor(
    val value: Int,
    val trueColor: Boolean = false,
)

/** The attributes applied to cells written after this state is selected. */
data class TerminalAttribute(
    val foreground: TerminalColor? = null,
    val background: TerminalColor? = null,
    val bold: Boolean = false,
    val underline: Boolean = false,
    val inverse: Boolean = false,
)

/** One cell in the terminal grid. A String allows a supplementary Unicode code point. */
data class TerminalCell(
    val character: String = " ",
    val attribute: TerminalAttribute = TerminalAttribute(),
)

/** A line snapshot, including its fixed-width cells. */
typealias TerminalLine = List<TerminalCell>

/** A small VT-style screen buffer with bounded normal-screen scrollback. */
class TerminalBuffer(
    var rows: Int = DEFAULT_ROWS,
    var columns: Int = DEFAULT_COLUMNS,
) {
    var cursorRow: Int = 0
        private set
    var cursorColumn: Int = 0
        private set
    var currentAttribute: TerminalAttribute = TerminalAttribute()
        private set
    var bracketedPaste: Boolean = false
        private set

    private var normalGrid = emptyGrid(rows, columns)
    private var alternateGrid: MutableList<MutableList<TerminalCell>>? = null
    private var usingAlternate = false
    private var normalCursorRow = 0
    private var normalCursorColumn = 0
    private val scrollback = ArrayDeque<TerminalLine>()

    /** Returns all normal-screen history followed by the current visible grid. */
    fun lines(): List<TerminalLine> =
        if (usingAlternate) {
            activeGrid().map { it.toList() }
        } else {
            scrollback.toList() + activeGrid().map { it.toList() }
        }

    fun clear() {
        currentAttribute = TerminalAttribute()
        scrollback.clear()
        activeGrid().forEach { row -> row.fill(TerminalCell()) }
        cursorRow = 0
        cursorColumn = 0
    }

    fun setCursor(
        row: Int,
        column: Int,
    ) {
        cursorRow = row.coerceIn(0, rows - 1)
        cursorColumn = column.coerceIn(0, columns)
    }

    fun moveCursor(
        rowDelta: Int,
        columnDelta: Int,
    ) {
        setCursor(cursorRow + rowDelta, cursorColumn + columnDelta)
    }

    fun setAttribute(attribute: TerminalAttribute) {
        currentAttribute = attribute
    }

    fun setBracketedPaste(enabled: Boolean) {
        bracketedPaste = enabled
    }

    fun write(text: String) {
        text.forEachCodePoint { codePoint -> writeCharacter(String(Character.toChars(codePoint))) }
    }

    fun carriageReturn() {
        cursorColumn = 0
    }

    fun lineFeed() {
        if (cursorRow == rows - 1) scrollUp() else cursorRow++
    }

    fun backspace() {
        cursorColumn = (cursorColumn - 1).coerceAtLeast(0)
    }

    fun tab() {
        val next = ((cursorColumn / TAB_SIZE) + 1) * TAB_SIZE
        cursorColumn = next.coerceAtMost(columns)
    }

    fun eraseDisplay(mode: Int) {
        when (mode) {
            0 -> {
                eraseLine(0)
                for (row in cursorRow + 1 until rows) activeGrid()[row].fill(blank())
            }
            1 -> {
                for (row in 0 until cursorRow) activeGrid()[row].fill(blank())
                eraseLine(1)
            }
            2 -> activeGrid().forEach { it.fill(TerminalCell(attribute = currentAttribute)) }
        }
    }

    fun eraseLine(mode: Int) {
        val line = activeGrid()[cursorRow]
        when (mode) {
            0 -> for (column in cursorColumn.coerceAtMost(columns) until columns) {
                line[column] = blank()
            }
            1 -> for (column in 0..cursorColumn.coerceAtMost(columns - 1)) {
                line[column] = blank()
            }
            2 -> line.fill(blank())
        }
    }

    fun resize(
        newRows: Int,
        newColumns: Int,
    ) {
        val safeRows = newRows.coerceAtLeast(1)
        val safeColumns = newColumns.coerceAtLeast(1)
        normalGrid = resizeGrid(normalGrid, safeRows, safeColumns)
        alternateGrid = alternateGrid?.let { resizeGrid(it, safeRows, safeColumns) }
        val resizedScrollback = scrollback.map { line -> resizeLine(line, safeColumns) }
        scrollback.clear()
        scrollback.addAll(resizedScrollback)
        rows = safeRows
        columns = safeColumns
        cursorRow = cursorRow.coerceIn(0, rows - 1)
        cursorColumn = cursorColumn.coerceIn(0, columns)
    }

    fun useAlternate(enabled: Boolean) {
        if (enabled == usingAlternate) return
        if (enabled) {
            normalCursorRow = cursorRow
            normalCursorColumn = cursorColumn
            alternateGrid = emptyGrid(rows, columns)
            usingAlternate = true
        } else {
            usingAlternate = false
            alternateGrid = null
            cursorRow = normalCursorRow.coerceIn(0, rows - 1)
            cursorColumn = normalCursorColumn.coerceIn(0, columns)
        }
    }

    private fun writeCharacter(character: String) {
        if (cursorColumn >= columns) {
            cursorColumn = 0
            lineFeed()
        }
        activeGrid()[cursorRow][cursorColumn] = TerminalCell(character, currentAttribute)
        cursorColumn++
    }

    private fun scrollUp() {
        val grid = activeGrid()
        if (!usingAlternate) {
            scrollback.addLast(grid.removeAt(0).toList())
            while (scrollback.size > MAX_SCROLLBACK) scrollback.removeFirst()
        } else {
            grid.removeAt(0)
        }
        grid.add(MutableList(columns) { blank() })
    }

    private fun blank() = TerminalCell(attribute = currentAttribute)

    private fun activeGrid(): MutableList<MutableList<TerminalCell>> =
        if (usingAlternate) alternateGrid ?: error("alternate grid missing") else normalGrid

    companion object {
        const val DEFAULT_ROWS = 24
        const val DEFAULT_COLUMNS = 80
        const val MAX_SCROLLBACK = 2_000
        private const val TAB_SIZE = 8
    }
}

private inline fun String.forEachCodePoint(action: (Int) -> Unit) {
    var index = 0
    while (index < length) {
        val codePoint = codePointAt(index)
        action(codePoint)
        index += Character.charCount(codePoint)
    }
}
