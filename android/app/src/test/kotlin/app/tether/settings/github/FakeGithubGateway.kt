package app.tether.settings.github

/**
 * A GitHub gateway for unit tests. Each call returns or throws at once, so a test controls time
 * with the dispatcher alone. Set a field to script the next answer; a failure is a
 * [GithubFailureException], as the real gateway throws.
 */
class FakeGithubGateway : GithubGateway {
    var status: GithubStatus = GithubStatus.SignedOut
    var statusFailure: GithubFailure? = null
    var statusCalls = 0

    var startFailure: GithubFailure? = null
    var grant: DeviceGrant = DeviceGrant("ABCD-1234", "https://github.com/login/device", 5, 900)
    var startCalls = 0

    val polls = ArrayDeque<GithubSignInPoll>()
    var pollFailure: GithubFailure? = null
    var pollCalls = 0

    var tokenFailure: GithubFailure? = null
    val tokensReceived = mutableListOf<String>()
    var tokenKindAfterSignIn: GithubTokenKind? = GithubTokenKind.Classic
    var canReadReposAfterSignIn = true
    var signOutCalls = 0

    var identityFailure: GithubFailure? = null
    val identitiesSaved = mutableListOf<Pair<String, String>>()

    var listFailure: GithubFailure? = null
    var pageSize = GITHUB_PAGE_SIZE
    val pagesRequested = mutableListOf<Int>()

    /** Runs each clone. It may call [onProgress], then returns the project ID or throws. */
    var cloneBehavior: (
        url: String,
        name: String?,
        history: CloneHistory,
        onProgress: (CloneProgress) -> Unit,
    ) -> String = { _, _, _, _ -> "project-1" }
    val clonesStarted = mutableListOf<Triple<String, String?, CloneHistory>>()
    var cancelCalls = 0

    override fun status(): GithubStatus {
        statusCalls++
        statusFailure?.let { throw GithubFailureException(it) }
        return status
    }

    override fun startDeviceFlow(): DeviceGrant {
        startCalls++
        startFailure?.let { throw GithubFailureException(it) }
        return grant
    }

    override fun pollDeviceFlow(): GithubSignInPoll {
        pollCalls++
        pollFailure?.let { throw GithubFailureException(it) }
        return polls.removeFirstOrNull() ?: GithubSignInPoll.Pending
    }

    override fun signInWithToken(token: String) {
        tokenFailure?.let { throw GithubFailureException(it) }
        tokensReceived += token
        status =
            status.copy(
                signedIn = true,
                tokenKind = tokenKindAfterSignIn,
                canReadRepos = canReadReposAfterSignIn,
            )
    }

    override fun signOut() {
        signOutCalls++
        status = GithubStatus.SignedOut
    }

    override fun setIdentity(
        name: String,
        email: String,
    ) {
        identityFailure?.let { throw GithubFailureException(it) }
        identitiesSaved += name to email
    }

    override fun listRepos(page: Int): List<GithubRepo> {
        pagesRequested += page
        listFailure?.let { throw GithubFailureException(it) }
        return repos(page)
    }

    override fun cloneRepository(
        url: String,
        name: String?,
        history: CloneHistory,
        onProgress: (CloneProgress) -> Unit,
    ): String {
        clonesStarted += Triple(url, name, history)
        return cloneBehavior(url, name, history, onProgress)
    }

    override fun cancelClone() {
        cancelCalls++
    }

    /** Repositories for [page]: a full page when [pageSize] is set, then an empty page. */
    var repoSource: (Int) -> List<GithubRepo> = { page ->
        if (page == 1) {
            (1..pageSize).map {
                GithubRepo(
                    "https://github.com/owner/repo-$it.git",
                    "main",
                    false,
                    10,
                )
            }
        } else {
            emptyList()
        }
    }

    private fun repos(page: Int): List<GithubRepo> = repoSource(page)
}
