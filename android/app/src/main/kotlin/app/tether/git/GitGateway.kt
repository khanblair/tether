package app.tether.git

import app.tether.core.generated.GitDiffSpecView
import app.tether.core.generated.GitRuntime
import app.tether.core.generated.GitRuntimeException
import app.tether.core.generated.SecretVault

/** How many commits the History tab asks for. The core accepts 1 to 50. */
const val GIT_HISTORY_LIMIT = 50

/**
 * What the Git screens need from the Rust core. Every call blocks, so callers run it off the main
 * thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface GitGateway {
    /** The checked-out branch name, or null when HEAD is detached. */
    fun currentBranch(projectId: String): String?

    /** Local branches in name order. Unborn branches with no commit are not listed. */
    fun branches(projectId: String): List<GitBranch>

    /** Changed paths with their index and worktree states, in path order. */
    fun status(projectId: String): List<ChangeEntry>

    /** Up to [limit] commits from HEAD, newest first. An unborn repository gives an empty list. */
    fun log(
        projectId: String,
        limit: Int,
    ): List<GitCommit>

    /** The unified diff text for [request]. The core may cut it and says so in the text. */
    fun diff(
        projectId: String,
        request: DiffRequest,
    ): String
}

/**
 * The real gateway over the UniFFI [GitRuntime]. Core errors become [GitFailureException] so the
 * screens see one type.
 */
class CoreGitGateway(
    private val runtime: Lazy<GitRuntime>,
) : GitGateway {
    private val git: GitRuntime by runtime

    override fun currentBranch(projectId: String): String? = guard { git.currentBranch(projectId) }

    override fun branches(projectId: String): List<GitBranch> =
        guard {
            git.gitBranches(projectId).map { view ->
                GitBranch(name = view.name, commit = view.commit?.ifEmpty { null })
            }
        }

    override fun status(projectId: String): List<ChangeEntry> =
        guard {
            git.gitStatus(projectId).map { view ->
                ChangeEntry(
                    path = view.path,
                    staged = changeKindOf(view.indexState),
                    unstaged = changeKindOf(view.worktreeState),
                )
            }
        }

    override fun log(
        projectId: String,
        limit: Int,
    ): List<GitCommit> =
        try {
            guard {
                git.gitLog(projectId, limit.toUInt()).map { view ->
                    GitCommit(
                        id = view.id,
                        subject = view.subject,
                        authorName = view.authorName,
                        committedSeconds = view.committerTime,
                    )
                }
            }
        } catch (failure: GitFailureException) {
            // The core reports an unborn HEAD as a missing reference. An unborn repository has
            // no history, so it is not a failure.
            if (failure.failure == GitFailure.RepositoryFailed && failure.isUnbornHead()) {
                emptyList()
            } else {
                throw failure
            }
        }

    override fun diff(
        projectId: String,
        request: DiffRequest,
    ): String =
        guard {
            git.gitDiff(
                projectId,
                GitDiffSpecView(
                    staged = request.staged,
                    path = request.path,
                    from = null,
                    to = null,
                ),
            )
        }

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: GitRuntimeException) {
            throw GitFailureException(failure.toGitFailure(), failure)
        }
}

/**
 * The Git gateway over the app's database and workspaces. The runtime opens on the first call, so
 * creating the gateway never blocks the UI.
 */
fun createGitGateway(
    databasePath: String,
    workspacesRoot: String,
    vault: SecretVault,
): GitGateway = CoreGitGateway(lazy { GitRuntime(databasePath, workspacesRoot, vault) })
