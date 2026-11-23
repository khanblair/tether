package app.tether.git

/**
 * One file in a unified diff. [path] is the new name, or the old name when the file was deleted.
 * [oldPath] is set only for a rename. [binary] files have no [hunks].
 */
data class DiffFile(
    val path: String,
    val oldPath: String?,
    val hunks: List<DiffHunk>,
    val added: Int,
    val removed: Int,
    val binary: Boolean,
)

/** One hunk. [header] is the whole `@@ ... @@` line, with any function name after it. */
data class DiffHunk(
    val header: String,
    val lines: List<DiffLineModel>,
)

/** The kind of one diff line. [Note] is a `\ No newline at end of file` marker. */
enum class DiffLineKind {
    Context,
    Added,
    Removed,
    Note,
}

/**
 * One line of a hunk. [oldNo] is set for context and removed lines, [newNo] for context and
 * added lines. [text] has no leading mark and no trailing carriage return.
 */
data class DiffLineModel(
    val kind: DiffLineKind,
    val oldNo: Int?,
    val newNo: Int?,
    val text: String,
)
