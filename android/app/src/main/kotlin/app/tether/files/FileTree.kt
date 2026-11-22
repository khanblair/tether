package app.tether.files

/** The path of the project root, which the core lists when it gets the empty string. */
const val ROOT_DIR: String = ""

/** What the cache knows about one folder's children. */
sealed interface DirState {
    /** The listing is on its way. */
    data object Loading : DirState

    /** The children, as the core returned them, folders first. */
    data class Loaded(
        val entries: List<FileEntry>,
        val truncated: Boolean,
    ) : DirState

    /** The listing failed. A retry asks the core again. */
    data object Failed : DirState
}

/** One row of the file tree, from top to bottom. [depth] is how far the row is indented. */
sealed interface TreeRow {
    val depth: Int

    /** A file or folder. [expanded] is true for a folder whose children are shown. */
    data class Entry(
        val entry: FileEntry,
        override val depth: Int,
        val expanded: Boolean,
    ) : TreeRow

    /** A folder whose children are on their way. */
    data class Loading(
        val dir: String,
        override val depth: Int,
    ) : TreeRow

    /** A folder whose listing failed, with a retry. */
    data class Failed(
        val dir: String,
        override val depth: Int,
    ) : TreeRow

    /** A folder with no visible children. */
    data class Empty(
        override val depth: Int,
    ) : TreeRow

    /** The note under a listing that the core cut short. */
    data class Truncated(
        override val depth: Int,
    ) : TreeRow
}

/**
 * The rows the tree shows. The root is always listed. A folder shows its children only when its
 * path is in [expanded], and its children come from [cache], so collapsing keeps them.
 */
fun visibleRows(
    cache: Map<String, DirState>,
    expanded: Set<String>,
): List<TreeRow> {
    val rows = mutableListOf<TreeRow>()
    addRows(rows, ROOT_DIR, depth = 0, cache = cache, expanded = expanded)
    return rows
}

private fun addRows(
    rows: MutableList<TreeRow>,
    dir: String,
    depth: Int,
    cache: Map<String, DirState>,
    expanded: Set<String>,
) {
    when (val state = cache[dir]) {
        null, DirState.Loading -> rows += TreeRow.Loading(dir, depth)
        DirState.Failed -> rows += TreeRow.Failed(dir, depth)
        is DirState.Loaded -> addLoaded(rows, state, depth, cache, expanded)
    }
}

private fun addLoaded(
    rows: MutableList<TreeRow>,
    state: DirState.Loaded,
    depth: Int,
    cache: Map<String, DirState>,
    expanded: Set<String>,
) {
    if (state.entries.isEmpty()) rows += TreeRow.Empty(depth)
    for (entry in state.entries) {
        val open = entry.isDir && entry.path in expanded
        rows += TreeRow.Entry(entry, depth, open)
        if (open) addRows(rows, entry.path, depth + 1, cache, expanded)
    }
    if (state.truncated) rows += TreeRow.Truncated(depth)
}
