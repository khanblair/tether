package app.tether.git

/** The three Git tabs, in the order the segmented control shows them. */
enum class GitTab {
    Changes,
    Branches,
    History,
}

/**
 * The state of one path in the index or the worktree. [Changed] covers a state name this build
 * does not know, so a newer core never breaks the screen.
 */
enum class ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
    Changed,
}

/**
 * One path with its two states. [staged] is the index state and [unstaged] the worktree state;
 * null means that side has no change. An untracked file has only [unstaged].
 */
data class ChangeEntry(
    val path: String,
    val staged: ChangeKind?,
    val unstaged: ChangeKind?,
)

/** A local branch. [commit] is null for an unborn branch that has no commit yet. */
data class GitBranch(
    val name: String,
    val commit: String?,
)

/** One commit from the history. [committedSeconds] is Unix time in seconds. */
data class GitCommit(
    val id: String,
    val subject: String,
    val authorName: String,
    val committedSeconds: Long,
) {
    /** The first [GIT_SHORT_ID_LENGTH] characters of the id, as Git prints them. */
    val shortId: String get() = id.take(GIT_SHORT_ID_LENGTH)
}

/** How many characters of a commit id the screens show. */
const val GIT_SHORT_ID_LENGTH = 7

/** Which diff to read: [staged] picks index against HEAD, [path] limits it to one file. */
data class DiffRequest(
    val staged: Boolean,
    val path: String?,
)

/** Why a Git call failed, in the terms the screens need. Never carries a path or core text. */
enum class GitFailure {
    /** The project folder is not a Git repository yet. */
    NotRepository,

    /** The project does not exist. */
    NotFound,

    /** The request was not valid. Not reachable from these screens. */
    InvalidInput,

    /** Local storage or the core runtime is not available. */
    Unavailable,

    /** Git could not read the repository. */
    RepositoryFailed,
}

/** A Git call failed with [failure]. The cause stays out of the message. */
class GitFailureException(
    val failure: GitFailure,
    cause: Throwable? = null,
) : Exception(cause)
