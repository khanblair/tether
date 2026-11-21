package app.tether.providers

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext

/**
 * Runs one blocking core call on [io] and returns its result or its failure. A cancelled
 * coroutine is rethrown, so the caller stops as usual.
 */
internal suspend fun <T> callCore(
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
