package app.tether.git

import app.tether.core.generated.GitRuntimeException

/** The core's detail text when HEAD points at a branch with no commit yet. */
internal const val UNBORN_HEAD_DETAIL = "git reference was not found"

/** The core's detail text when the project folder is not a Git repository. */
internal const val NOT_A_REPOSITORY_DETAIL = "path is not a git repository"

/**
 * The screen failure for one core error. Every variant maps, so the `when` has no fallback. The
 * account and GitHub variants cannot come from these screens; they map to [GitFailure.Unavailable].
 */
internal fun GitRuntimeException.toGitFailure(): GitFailure =
    when (this) {
        is GitRuntimeException.NotFound -> GitFailure.NotFound
        is GitRuntimeException.InvalidInput -> GitFailure.InvalidInput
        is GitRuntimeException.Unavailable -> GitFailure.Unavailable
        is GitRuntimeException.GithubNotConfigured -> GitFailure.Unavailable
        is GitRuntimeException.NotSignedIn -> GitFailure.Unavailable
        is GitRuntimeException.NoDeviceGrant -> GitFailure.Unavailable
        is GitRuntimeException.GithubCredentials -> GitFailure.Unavailable
        is GitRuntimeException.GithubResponse -> GitFailure.Unavailable
        is GitRuntimeException.GithubNetwork -> GitFailure.Unavailable
        is GitRuntimeException.MissingScope -> GitFailure.Unavailable
        is GitRuntimeException.Repository -> repositoryFailure(detail)
    }

/** A repository error is "not a repository" when the core says so, and a failure otherwise. */
internal fun repositoryFailure(detail: String): GitFailure =
    if (detail.contains(NOT_A_REPOSITORY_DETAIL)) {
        GitFailure.NotRepository
    } else {
        GitFailure.RepositoryFailed
    }

/** True when a [GitFailureException] came from an unborn HEAD. */
internal fun GitFailureException.isUnbornHead(): Boolean =
    (cause as? GitRuntimeException.Repository)?.detail == UNBORN_HEAD_DETAIL

/** The failure a caught error stands for. Any other error counts as unavailable. */
internal fun Throwable.gitFailure(): GitFailure =
    when (this) {
        is GitFailureException -> failure
        is GitRuntimeException -> toGitFailure()
        else -> GitFailure.Unavailable
    }

/**
 * The change kind for a core state name, such as "Added". An empty name means no change on that
 * side. A name this build does not know maps to [ChangeKind.Changed].
 */
internal fun changeKindOf(name: String): ChangeKind? =
    when {
        name.isEmpty() -> null
        else ->
            ChangeKind.entries.firstOrNull { it != ChangeKind.Changed && it.name == name }
                ?: ChangeKind.Changed
    }
