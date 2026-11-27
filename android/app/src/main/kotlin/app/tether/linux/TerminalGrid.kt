package app.tether.linux

// Stateless grid helpers for TerminalBuffer. They read no buffer state, so they live
// outside the class to keep TerminalBuffer's function count within the detekt limit.

internal fun emptyGrid(
    rows: Int,
    columns: Int,
): MutableList<MutableList<TerminalCell>> =
    MutableList(rows.coerceAtLeast(1)) { MutableList(columns.coerceAtLeast(1)) { TerminalCell() } }

internal fun resizeGrid(
    old: MutableList<MutableList<TerminalCell>>,
    newRows: Int,
    newColumns: Int,
): MutableList<MutableList<TerminalCell>> =
    MutableList(newRows) { row ->
        resizeLine(old.getOrNull(row).orEmpty(), newColumns).toMutableList()
    }

internal fun resizeLine(
    old: List<TerminalCell>,
    newColumns: Int,
): List<TerminalCell> = List(newColumns) { column -> old.getOrNull(column) ?: TerminalCell() }
