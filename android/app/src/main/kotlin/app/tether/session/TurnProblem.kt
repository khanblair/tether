package app.tether.session

import app.tether.core.generated.CoreErrorView
import app.tether.core.generated.ErrorKindView

/** The block a failed turn shows. Each one names the cause and offers the fix that exists. */
enum class TurnProblem { KeyRejected, RateLimited, Network, Other }

/** Maps the core's error kind to the block the transcript shows for a failed turn. */
internal fun turnProblemOf(error: CoreErrorView): TurnProblem =
    when (error.kind) {
        ErrorKindView.AUTH -> TurnProblem.KeyRejected
        ErrorKindView.RATE_LIMITED -> TurnProblem.RateLimited
        ErrorKindView.NETWORK -> TurnProblem.Network
        ErrorKindView.INTERNAL,
        ErrorKindView.INVALID_INPUT,
        ErrorKindView.NOT_FOUND,
        ErrorKindView.CANCELLED,
        -> TurnProblem.Other
    }

/**
 * Whether the block offers a retry. A rate limit always does, because the wait is the user's
 * choice. Network and other failures do only when the core says the same request can succeed.
 */
internal fun TurnProblem.offersRetry(error: CoreErrorView): Boolean =
    when (this) {
        TurnProblem.RateLimited -> true
        TurnProblem.Network, TurnProblem.Other -> error.retryable
        TurnProblem.KeyRejected -> false
    }
