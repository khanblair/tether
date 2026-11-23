package app.tether.git

/**
 * A Git gateway for unit tests. Each call returns or throws at once. Set a field to script the
 * next answer; a failure is a [GitFailureException], as the real gateway throws. Calls are
 * counted and the diff requests are recorded, so tests can check what was read.
 */
class FakeGitGateway : GitGateway {
    var branch: String? = "main"
    var branchFailure: GitFailure? = null
    var branchCalls = 0

    var listed: List<GitBranch> = listOf(GitBranch("main", "3f2a9c1d0e4b"))
    var branchesFailure: GitFailure? = null
    var branchesCalls = 0

    var entries: List<ChangeEntry> = emptyList()
    var statusFailure: GitFailure? = null
    var statusCalls = 0

    var commits: List<GitCommit> = emptyList()
    var logFailure: GitFailure? = null
    val logLimits = mutableListOf<Int>()

    var diffText: String = ""
    var diffFailure: GitFailure? = null
    val diffRequests = mutableListOf<DiffRequest>()

    override fun currentBranch(projectId: String): String? {
        branchCalls++
        branchFailure?.let { throw GitFailureException(it) }
        return branch
    }

    override fun branches(projectId: String): List<GitBranch> {
        branchesCalls++
        branchesFailure?.let { throw GitFailureException(it) }
        return listed
    }

    override fun status(projectId: String): List<ChangeEntry> {
        statusCalls++
        statusFailure?.let { throw GitFailureException(it) }
        return entries
    }

    override fun log(
        projectId: String,
        limit: Int,
    ): List<GitCommit> {
        logLimits += limit
        logFailure?.let { throw GitFailureException(it) }
        return commits
    }

    override fun diff(
        projectId: String,
        request: DiffRequest,
    ): String {
        diffRequests += request
        diffFailure?.let { throw GitFailureException(it) }
        return diffText
    }
}
