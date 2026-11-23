package app.tether.git

/** One piece of screen data: not loaded yet, loading, loaded, or failed with a reason. */
sealed interface Loadable<out T> {
    /** Not asked for yet. A tab loads the first time it is opened. */
    data object Idle : Loadable<Nothing>

    /** The read is running. */
    data object Loading : Loadable<Nothing>

    /** The read finished. */
    data class Ready<T>(
        val value: T,
    ) : Loadable<T>

    /** The read failed. The screen shows [failure] and a Try again action. */
    data class Failed(
        val failure: GitFailure,
    ) : Loadable<Nothing>
}

/**
 * The Changes tab. [branch] is the checked-out branch, or null when HEAD is detached. [staged]
 * and [unstaged] hold the entries that have a change on that side, in path order.
 */
data class ChangesContent(
    val branch: String?,
    val staged: List<ChangeEntry>,
    val unstaged: List<ChangeEntry>,
)

/** The Branches tab. [current] is the checked-out branch, or null when HEAD is detached. */
data class BranchesContent(
    val current: String?,
    val branches: List<GitBranch>,
)

/** The History tab. [nowMillis] is the clock reading when the list was read, for relative times. */
data class HistoryContent(
    val commits: List<GitCommit>,
    val nowMillis: Long,
)

/** What the Git screen shows: the open tab and the data of each tab. */
data class GitUiState(
    val tab: GitTab = GitTab.Changes,
    val changes: Loadable<ChangesContent> = Loadable.Idle,
    val branches: Loadable<BranchesContent> = Loadable.Idle,
    val history: Loadable<HistoryContent> = Loadable.Idle,
)

/** The parsed diff of one read. [truncated] is true when the core cut the text at its limit. */
data class DiffContent(
    val files: List<DiffFile>,
    val truncated: Boolean,
)

/**
 * What the diff screen shows. [path] is the one file, or null for all changes. [staged] picks the
 * index side. [wrap] is true when long lines wrap instead of scrolling sideways.
 */
data class DiffUiState(
    val path: String?,
    val staged: Boolean,
    val wrap: Boolean = true,
    val content: Loadable<DiffContent> = Loadable.Idle,
)
