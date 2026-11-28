package app.tether.nav

import kotlinx.coroutines.CancellationException

/**
 * Ends a session screen: when the session has no message, [delete] removes it, and [close] always
 * runs after, even when a check or the delete fails. A failure goes to [failed] and never stops the
 * close. A cancellation is rethrown after the close.
 */
@Suppress("TooGenericExceptionCaught")
internal fun closeDiscardingEmpty(
    isEmpty: () -> Boolean,
    delete: () -> Unit,
    close: () -> Unit,
    failed: (Exception) -> Unit,
) {
    try {
        if (isEmpty()) delete()
    } catch (cancelled: CancellationException) {
        throw cancelled
    } catch (error: Exception) {
        failed(error)
    } finally {
        close()
    }
}
