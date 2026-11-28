package app.tether.settings.github

import app.tether.core.generated.CloneProgressView
import app.tether.core.generated.GitRuntime
import app.tether.core.generated.GitRuntimeException
import app.tether.core.generated.SecretVault

/**
 * What the GitHub screens need from the Rust core. Every call blocks, so callers run it off the
 * main thread. [cloneRepository] may report progress on any thread. A fake implements this in
 * unit tests, so no test loads the native library.
 */
interface GithubGateway {
    /** The saved account and its scopes. */
    fun status(): GithubStatus

    /** Starts the device flow and returns the code. Fails as NotConfigured with no client ID. */
    fun startDeviceFlow(): DeviceGrant

    /** Polls the active device grant once. */
    fun pollDeviceFlow(): GithubSignInPoll

    /** Validates and saves a personal access token. */
    fun signInWithToken(token: String)

    /** Removes the saved token and the cached GitHub identity. */
    fun signOut()

    /** Saves the Git author name and email for commits. */
    fun setIdentity(
        name: String,
        email: String,
    )

    /** One page of the signed-in account's repositories. Pages start at 1. */
    fun listRepos(page: Int): List<GithubRepo>

    /** Clones [url] into a new project and returns its ID. Blocks until done or cancelled. */
    fun cloneRepository(
        url: String,
        name: String?,
        history: CloneHistory,
        onProgress: (CloneProgress) -> Unit,
    ): String

    /** Cancels the clone in flight, if any. Safe to call when none runs. */
    fun cancelClone()
}

/**
 * The real gateway over the UniFFI [GitRuntime]. The runtime opens on the first call, off the
 * main thread. Core errors become [GithubFailureException] so the screens see one type.
 */
class CoreGithubGateway(
    private val runtime: Lazy<GitRuntime>,
) : GithubGateway {
    private val git: GitRuntime by runtime

    override fun status(): GithubStatus =
        guard {
            git.githubStatus().toGithubStatus()
        }

    override fun startDeviceFlow(): DeviceGrant =
        guard {
            git.githubSignInStart().let { view ->
                DeviceGrant(
                    userCode = view.userCode,
                    verificationUri = view.verificationUri,
                    intervalSeconds = view.intervalSeconds.toLong(),
                    expiresInSeconds = view.expiresInSeconds.toLong(),
                )
            }
        }

    override fun pollDeviceFlow(): GithubSignInPoll =
        guard { git.githubSignInPoll().toSignInPoll() }

    override fun signInWithToken(token: String) = guard { git.githubSignInWithToken(token) }

    override fun signOut() = guard { git.githubSignOut() }

    override fun setIdentity(
        name: String,
        email: String,
    ) = guard { git.setGitIdentity(name, email) }

    override fun listRepos(page: Int): List<GithubRepo> =
        guard {
            git.listGithubRepos(page.toUInt()).map { view ->
                GithubRepo(
                    cloneUrl = view.cloneUrl,
                    defaultBranch = view.defaultBranch,
                    isPrivate = view.`private`,
                    sizeKb = view.sizeKb.toLong(),
                )
            }
        }

    override fun cloneRepository(
        url: String,
        name: String?,
        history: CloneHistory,
        onProgress: (CloneProgress) -> Unit,
    ): String =
        guard {
            val progress =
                object : CloneProgressView {
                    override fun onProgress(
                        phase: String,
                        received: ULong,
                        total: ULong?,
                    ) {
                        onProgress(
                            CloneProgress(
                                phase = phaseStepOf(phase),
                                received = received.toLong(),
                                total = total?.toLong(),
                            ),
                        )
                    }
                }
            git.cloneRepository(url, name, history.toShallowChoice(), progress)
        }

    override fun cancelClone() = guard { git.cancelClone() }

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: GitRuntimeException) {
            throw GithubFailureException(failure.toGithubFailure(), failure)
        }
}

/**
 * The GitHub gateway over the app's database and workspaces, with the token vault. The runtime
 * opens on the first call, so creating the gateway never blocks the UI. [clientId] is the public
 * client ID of the GitHub OAuth app. With it, Sign in with GitHub works; without it the runtime
 * reports that sign-in is not set up and the person uses a token.
 */
fun createGithubGateway(
    databasePath: String,
    workspacesRoot: String,
    vault: SecretVault,
    clientId: String = "",
): GithubGateway =
    CoreGithubGateway(
        lazy {
            GitRuntime(databasePath, workspacesRoot, vault).also { runtime ->
                configureDeviceFlow(clientId, runtime::configureGithub)
            }
        },
    )

/** Gives the runtime the OAuth client ID when the build has one. A blank ID changes nothing. */
internal fun configureDeviceFlow(
    clientId: String,
    configure: (String) -> Unit,
) {
    val id = clientId.trim()
    if (id.isNotEmpty()) configure(id)
}
