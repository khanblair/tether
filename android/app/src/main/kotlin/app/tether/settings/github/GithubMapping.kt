package app.tether.settings.github

import app.tether.core.generated.GitRuntimeException
import app.tether.core.generated.ShallowChoiceView
import app.tether.core.generated.SignInStateView

/** The screen failure for one core error. Every variant maps, so the `when` has no fallback. */
internal fun GitRuntimeException.toGithubFailure(): GithubFailure =
    when (this) {
        is GitRuntimeException.NotFound -> GithubFailure.NotFound
        is GitRuntimeException.InvalidInput -> GithubFailure.InvalidInput
        is GitRuntimeException.Unavailable -> GithubFailure.Unavailable
        is GitRuntimeException.GithubNotConfigured -> GithubFailure.NotConfigured
        is GitRuntimeException.NotSignedIn -> GithubFailure.NotSignedIn
        is GitRuntimeException.NoDeviceGrant -> GithubFailure.NoDeviceGrant
        is GitRuntimeException.GithubCredentials -> GithubFailure.Rejected
        is GitRuntimeException.GithubResponse -> GithubFailure.BadResponse
        is GitRuntimeException.GithubNetwork -> GithubFailure.Network
        is GitRuntimeException.MissingScope -> GithubFailure.MissingScope
        is GitRuntimeException.Repository -> GithubFailure.RepositoryFailed
    }

/** The failure a caught error stands for. Any other error counts as unavailable. */
internal fun Throwable.githubFailure(): GithubFailure =
    when (this) {
        is GithubFailureException -> failure
        is GitRuntimeException -> toGithubFailure()
        else -> GithubFailure.Unavailable
    }

/** The poll result the screens use. Every core case maps. */
internal fun SignInStateView.toSignInPoll(): GithubSignInPoll =
    when (this) {
        is SignInStateView.Pending -> GithubSignInPoll.Pending
        is SignInStateView.SlowDown -> GithubSignInPoll.SlowDown
        is SignInStateView.Authorized -> GithubSignInPoll.Authorized
        is SignInStateView.Denied -> GithubSignInPoll.Denied
        is SignInStateView.Expired -> GithubSignInPoll.Expired
    }

/** The core's clone history enum for a screen choice. */
internal fun CloneHistory.toShallowChoice(): ShallowChoiceView =
    when (this) {
        CloneHistory.Auto -> ShallowChoiceView.AUTO
        CloneHistory.Shallow -> ShallowChoiceView.SHALLOW
        CloneHistory.Full -> ShallowChoiceView.FULL
    }

/** The phase step for the phase name the core reports (the Rust debug name of its enum). */
internal fun phaseStepOf(phase: String): ClonePhaseStep =
    when (phase) {
        "Resolving" -> ClonePhaseStep.Resolving
        "Receiving" -> ClonePhaseStep.Receiving
        "CheckingOut" -> ClonePhaseStep.CheckingOut
        else -> ClonePhaseStep.Unknown
    }
