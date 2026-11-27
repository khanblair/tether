package app.tether.settings.github

/** Repositories per page. The core asks GitHub for this many at a time. */
const val GITHUB_PAGE_SIZE = 30

/** The saved GitHub account. [login], [name], and [email] are null when GitHub did not say. */
data class GithubStatus(
    val signedIn: Boolean,
    val login: String?,
    val name: String?,
    val email: String?,
    val scopes: List<String>,
    val tokenKind: GithubTokenKind? = null,
    val canReadRepos: Boolean = false,
) {
    companion object {
        /** No account saved. */
        val SignedOut = GithubStatus(false, null, null, null, emptyList())
    }
}

/** The kind of personal access token saved for the account. */
enum class GithubTokenKind {
    /** A classic personal access token with named scopes. */
    Classic,

    /** A fine-grained personal access token with repository permissions. */
    FineGrained,
}

/** Whether the saved token can read the account's repositories. */
enum class GithubRepositoryAccess {
    Yes,
    No,
}

/** How the account card explains the token's access settings. */
enum class GithubScopePresentation {
    ClassicScopes,
    FineGrained,
    Unknown,
}

/** The pure display decisions for the signed-in account card. */
data class GithubAccountAccess(
    val tokenKind: GithubTokenKind?,
    val repositoryAccess: GithubRepositoryAccess,
    val scopes: List<String>,
    val scopePresentation: GithubScopePresentation,
)

/** Chooses the account card's access text and scope presentation. */
fun GithubStatus.accountAccess(): GithubAccountAccess =
    GithubAccountAccess(
        tokenKind = tokenKind,
        repositoryAccess =
            if (canReadRepos) GithubRepositoryAccess.Yes else GithubRepositoryAccess.No,
        scopes = if (tokenKind == GithubTokenKind.Classic) scopes else emptyList(),
        scopePresentation =
            when (tokenKind) {
                GithubTokenKind.Classic -> GithubScopePresentation.ClassicScopes
                GithubTokenKind.FineGrained -> GithubScopePresentation.FineGrained
                null -> GithubScopePresentation.Unknown
            },
    )

/** The one-time code of the device flow. [intervalSeconds] is the minimum wait between polls. */
data class DeviceGrant(
    val userCode: String,
    val verificationUri: String,
    val intervalSeconds: Long,
    val expiresInSeconds: Long,
)

/** One result of a device-flow poll, as the screens use it. */
sealed interface GithubSignInPoll {
    /** The user has not approved the code yet. */
    data object Pending : GithubSignInPoll

    /** GitHub asked for a longer wait between polls. */
    data object SlowDown : GithubSignInPoll

    /** The user approved. The token is saved in the core and never reaches the screens. */
    data object Authorized : GithubSignInPoll

    /** The user denied access. */
    data object Denied : GithubSignInPoll

    /** The code expired before the user approved it. */
    data object Expired : GithubSignInPoll
}

/** One repository from the signed-in account. The core gives only the clone URL, not a name. */
data class GithubRepo(
    val cloneUrl: String,
    val defaultBranch: String,
    val isPrivate: Boolean,
    val sizeKb: Long,
)

/** The clone history choice. [Auto] lets the core pick shallow or full from the repository size. */
enum class CloneHistory {
    Auto,
    Shallow,
    Full,
}

/** The clone phases the core reports. [Unknown] covers any phase name this build does not know. */
enum class ClonePhaseStep {
    Resolving,
    Receiving,
    CheckingOut,
    Unknown,
}

/** One progress report. [received] and [total] are bytes; [total] is null when unknown. */
data class CloneProgress(
    val phase: ClonePhaseStep,
    val received: Long,
    val total: Long?,
)

/** Why a GitHub call failed, in the terms the screens need. Never carries a token or a path. */
enum class GithubFailure {
    /** The input was empty or contained a character the core refuses. */
    InvalidInput,

    /** The project or repository does not exist. */
    NotFound,

    /** Local storage or the runtime is not available. */
    Unavailable,

    /** The app has no GitHub client ID, so the device flow cannot start. */
    NotConfigured,

    /** No GitHub token is saved. */
    NotSignedIn,

    /** There is no device code to poll. */
    NoDeviceGrant,

    /** GitHub rejected the token or the account. */
    Rejected,

    /** GitHub sent a reply the app cannot read. */
    BadResponse,

    /** The request to GitHub did not finish. */
    Network,

    /** The token lacks the repo scope. */
    MissingScope,

    /** The clone or the repository check failed. */
    RepositoryFailed,
}

/** A GitHub call failed with [failure]. The cause stays out of the message. */
class GithubFailureException(
    val failure: GithubFailure,
    cause: Throwable? = null,
) : Exception(cause)
