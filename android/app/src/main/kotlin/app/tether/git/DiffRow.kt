package app.tether.git

/**
 * One row of the diff list. The diff view shows one lazy list of these rows, so every file, hunk,
 * and line is its own item and only the visible ones are composed.
 */
sealed interface DiffRow {
    /** The header of file [fileIndex]: its path and counts. */
    data class FileHeader(
        val fileIndex: Int,
        val file: DiffFile,
    ) : DiffRow

    /** A binary file. It has no lines to show. */
    data class Binary(
        val fileIndex: Int,
    ) : DiffRow

    /** A file with no text change, such as a mode change only. */
    data class NoText(
        val fileIndex: Int,
    ) : DiffRow

    /** The `@@ ... @@` line that opens a hunk. */
    data class HunkHeader(
        val header: String,
    ) : DiffRow

    /** One line of a hunk. */
    data class Line(
        val line: DiffLineModel,
    ) : DiffRow
}

/** Flattens [files] into the rows the list shows, in order. */
fun flattenDiff(files: List<DiffFile>): List<DiffRow> =
    files.flatMapIndexed { index, file ->
        val body =
            when {
                file.binary -> listOf(DiffRow.Binary(index))
                file.hunks.isEmpty() -> listOf(DiffRow.NoText(index))
                else -> file.hunks.flatMap { hunk -> hunkRows(hunk) }
            }
        listOf<DiffRow>(DiffRow.FileHeader(index, file)) + body
    }

/** The position of file [fileIndex] in [rows], or -1 when it is not there. */
fun rowIndexOfFile(
    rows: List<DiffRow>,
    fileIndex: Int,
): Int = rows.indexOfFirst { it is DiffRow.FileHeader && it.fileIndex == fileIndex }

private fun hunkRows(hunk: DiffHunk): List<DiffRow> =
    listOf<DiffRow>(DiffRow.HunkHeader(hunk.header)) + hunk.lines.map { DiffRow.Line(it) }
