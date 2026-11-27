package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.MessageView
import app.tether.core.generated.ProcessView
import app.tether.core.generated.ProviderView
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
    /** The model's display name, or its raw id when the name is not known. */
    val modelName: String,
    /** Whether the model reads images. Unknown counts as no. */
    val supportsImages: Boolean,
    val usage: SessionUsage,
    /** The background processes present when this snapshot was read. */
    val processes: List<ProcessView>,
    /** Whether Bypass is on in the open session. A failed read counts as off. */
    val bypass: Boolean,
)

/** The status the line shows: the saved mode, or Bypass while it is on in the open session. */
internal fun SessionStatus.shownWith(bypass: Boolean): SessionStatus =
    if (bypass) copy(mode = BYPASS_STATUS_MODE) else this

/** The mode the composer chip shows: Bypass while it is on, else the saved mode. */
internal fun composerMode(snapshot: TranscriptSnapshot?): SessionMode =
    when {
        snapshot == null -> SessionMode.ACCEPT_EDITS
        snapshot.bypass -> SessionMode.BYPASS
        else -> snapshot.details.mode
    }

/** The status mode name the status line reads as Bypass. */
private const val BYPASS_STATUS_MODE = "bypass"

/** The display name of the model with [id] in [models], or the id itself when it is not there. */
internal fun modelDisplayName(
    id: String,
    models: List<ModelChoice>,
): String = models.firstOrNull { it.id == id }?.displayName ?: id

/**
 * Remembers the model found for the last provider and id, so a streamed reply, which reloads the
 * snapshot on every event, does not read the whole model list each time. A failed read is not
 * remembered, so the next reload tries again.
 */
internal class ModelLookup {
    private var key: Pair<ProviderView, String>? = null
    private var found: ModelChoice? = null

    @Synchronized
    fun find(
        provider: ProviderView,
        id: String,
        load: () -> List<ModelChoice>,
    ): ModelChoice? {
        val wanted = provider to id
        if (key != wanted) {
            runCatching(load).getOrNull()?.let { models ->
                found = models.firstOrNull { it.id == id }
                key = wanted
            }
        }
        return if (key == wanted) found else null
    }
}

internal suspend fun SessionConnection.snapshot(): TranscriptSnapshot =
    withContext(Dispatchers.IO) {
        val details = settings.details(sessionId)
        val model =
            modelLookup.find(details.provider, details.modelId) {
                settings.models(details.provider)
            }
        TranscriptSnapshot(
            messages = transcript().filterNot(MessageView::rewound).sortedBy(MessageView::seq),
            status = status(),
            details = details,
            modelName = modelDisplayName(details.modelId, listOfNotNull(model)),
            supportsImages = model?.images == true,
            usage = settings.usage(sessionId),
            processes = listProcesses(),
            bypass =
                sessionCall(Dispatchers.IO) { settings.bypassActive(sessionId) }
                    .getOrDefault(false),
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
