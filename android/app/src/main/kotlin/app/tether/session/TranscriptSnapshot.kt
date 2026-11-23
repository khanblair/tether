package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.MessageView
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The session's messages and status, as the core saved them. Every core event reloads it, so a
 * streamed reply grows in place and a finished turn shows its saved text.
 */
internal data class TranscriptSnapshot(
    val messages: List<MessageView>,
    val status: SessionStatus,
    val details: SessionDetails,
    val usage: SessionUsage,
)

internal suspend fun SessionConnection.snapshot(): TranscriptSnapshot =
    withContext(Dispatchers.IO) {
        TranscriptSnapshot(
            messages = transcript().filterNot(MessageView::rewound).sortedBy(MessageView::seq),
            status = status(),
            details = settings.details(sessionId),
            usage = settings.usage(sessionId),
        )
    }

/** Applies each core event and reloads the snapshot. A failed reload marks the screen failed. */
@Composable
internal fun CoreEvents(
    connection: SessionConnection?,
    onEvent: (CoreEventView) -> Unit,
    onLoaded: (TranscriptSnapshot) -> Unit,
    onFailed: () -> Unit,
) {
    LaunchedEffect(connection) {
        if (connection != null) {
            try {
                coroutineScope {
                    // Listen before the first read, so an event sent during the read is not lost.
                    launch(start = CoroutineStart.UNDISPATCHED) {
                        connection.changes.collect { event ->
                            onEvent(event)
                            onLoaded(connection.snapshot())
                        }
                    }
                    onLoaded(connection.snapshot())
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                onFailed()
            }
        }
    }
}

/** Reloads the transcript after a sheet changed the session. The first load is in the screen. */
@Composable
internal fun SnapshotReload(
    connection: SessionConnection?,
    reloads: Int,
    onLoaded: (TranscriptSnapshot) -> Unit,
    onFailed: () -> Unit,
) {
    LaunchedEffect(connection, reloads) {
        if (connection != null && reloads > 0) {
            try {
                onLoaded(connection.snapshot())
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                onFailed()
            }
        }
    }
}
