package app.tether.session

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext

/**
 * Runs one blocking session call on [io] and returns its result. A failure comes back in the
 * [Result], so each view model decides what to show. Cancellation is never caught.
 */
internal suspend fun <T> sessionCall(
    io: CoroutineDispatcher,
    block: () -> T,
): Result<T> =
    try {
        Result.success(withContext(io) { block() })
    } catch (cancelled: CancellationException) {
        throw cancelled
    } catch (
        @Suppress("TooGenericExceptionCaught") failure: Exception,
    ) {
        Result.failure(failure)
    }

/** The reason a failed session call gave, or [SessionFailure.OTHER] for an unexpected error. */
internal fun Throwable.sessionFailure(): SessionFailure =
    (this as? SessionSettingsException)?.failure ?: SessionFailure.OTHER
