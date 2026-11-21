package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton
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
private data class TranscriptSnapshot(
    val messages: List<MessageView>,
    val status: SessionStatus,
    val details: SessionDetails,
    val usage: SessionUsage,
)

private suspend fun SessionConnection.snapshot(): TranscriptSnapshot =
    withContext(Dispatchers.IO) {
        TranscriptSnapshot(
            messages = transcript().filterNot(MessageView::rewound).sortedBy(MessageView::seq),
            status = status(),
            details = settings.details(sessionId),
            usage = settings.usage(sessionId),
        )
    }

/** Persisted messages are read off the main thread when the session route opens. */
@Composable
fun TranscriptScreen(
    connection: SessionConnection?,
    onBack: () -> Unit,
    sheets: SessionSheetEnv? = null,
) {
    var snapshot by remember(connection) { mutableStateOf<TranscriptSnapshot?>(null) }
    var failed by remember(connection) { mutableStateOf(false) }
    var running by remember(connection) { mutableStateOf(false) }
    var proposedTool by remember(connection) { mutableStateOf<Pair<String, String>?>(null) }
    var turnError by remember(connection) { mutableStateOf<String?>(null) }
    val sheetControl = remember(connection) { SessionSheetControl() }
    val turnFailed = stringResource(R.string.transcript_turn_failed)
    val sendFailed = stringResource(R.string.transcript_send_failed)
    SnapshotReload(
        connection = connection,
        reloads = sheetControl.reloads,
        onLoaded = { snapshot = it },
        onFailed = { failed = true },
    )
    LaunchedEffect(connection) {
        if (connection != null) {
            try {
                coroutineScope {
                    // Listen before the first read, so an event sent during the read is not lost.
                    launch(start = CoroutineStart.UNDISPATCHED) {
                        connection.changes.collect { event ->
                            if (event.endsTurn()) running = false
                            proposedTool = event.proposedToolOr(proposedTool)
                            event.turnFailureOr(turnFailed)?.let { turnError = it }
                            snapshot = connection.snapshot()
                        }
                    }
                    snapshot = connection.snapshot()
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                failed = true
            }
        }
    }
    TranscriptContent(
        messages = snapshot?.messages.orEmpty(),
        failed = failed,
        title = snapshot?.details?.title ?: stringResource(R.string.route_session_title),
        status = snapshot?.status,
        usage = snapshot?.usage,
        proposedTool = proposedTool,
        turnError = turnError,
        onBack = onBack,
        onModelClick = sheetOpener(sheets, sheetControl, SessionSheetKind.Model),
        onModeClick = sheetOpener(sheets, sheetControl, SessionSheetKind.Mode),
        onMore = sheetOpener(sheets, sheetControl, SessionSheetKind.Menu),
        composer =
            connection?.let { active ->
                {
                    ComposerControls(
                        connection = active,
                        modelId = snapshot?.details?.modelId.orEmpty(),
                        running = running,
                        onSendStart = {
                            running = true
                            turnError = null
                            proposedTool = null
                        },
                        onSendFailed = {
                            running = false
                            turnError = sendFailed
                        },
                    )
                }
            },
    )
    SessionSheetsSlot(
        connection = connection,
        sheets = sheets,
        control = sheetControl,
        isRunning = { running },
    )
}

/** Reloads the transcript after a sheet changed the session. The first load is in the screen. */
@Composable
private fun SnapshotReload(
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

/** A sheet opener for a session with sheets. Without sheets, the control has no action. */
private fun sheetOpener(
    sheets: SessionSheetEnv?,
    control: SessionSheetControl,
    kind: SessionSheetKind,
): (() -> Unit)? = sheets?.let { { control.openSheet(kind) } }

@Composable
private fun ComposerControls(
    connection: SessionConnection,
    modelId: String,
    running: Boolean,
    onSendStart: () -> Unit,
    onSendFailed: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    Composer(
        modelId = modelId,
        running = running,
        onSend = { text ->
            onSendStart()
            scope.launch {
                try {
                    withContext(Dispatchers.IO) { connection.send(text) }
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (_: Exception) {
                    onSendFailed()
                }
            }
        },
        onStop = { scope.launch(Dispatchers.IO) { connection.stop() } },
    )
}

/** Lazy items use persisted IDs so rows keep their identity as text arrives. */
@Composable
fun TranscriptContent(
    messages: List<MessageView>,
    failed: Boolean,
    title: String,
    status: SessionStatus? = null,
    usage: SessionUsage? = null,
    proposedTool: Pair<String, String>? = null,
    turnError: String? = null,
    onBack: () -> Unit,
    onModelClick: (() -> Unit)? = null,
    onModeClick: (() -> Unit)? = null,
    onMore: (() -> Unit)? = null,
    composer: (@Composable () -> Unit)? = null,
) {
    val listState = rememberLazyListState()
    FollowTail(listState, messages)
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = title,
                navigationIcon = { BackButton(onBack) },
                actions = {
                    if (onMore != null) {
                        TetherTextButton(
                            label = stringResource(R.string.session_more),
                            onClick = onMore,
                        )
                    }
                },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            Box(modifier = Modifier.weight(1f)) {
                if (messages.isEmpty()) {
                    val emptyLabel =
                        if (failed) R.string.transcript_load_failed else R.string.transcript_empty
                    Text(
                        text = stringResource(emptyLabel),
                        color = TetherTheme.colors.text2,
                        modifier = Modifier.padding(TetherSpace.space4),
                    )
                } else {
                    LazyColumn(
                        state = listState,
                        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier.fillMaxSize(),
                    ) {
                        items(messages, key = MessageView::id) { message ->
                            TranscriptMessage(
                                message,
                                Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                            )
                        }
                    }
                }
            }
            proposedTool?.let { (tool, target) ->
                Text(
                    text = "${stringResource(R.string.transcript_tool)} · $tool: $target",
                    style = MaterialTheme.typography.labelMedium,
                    color = TetherTheme.colors.text2,
                    modifier =
                        Modifier
                            .fillMaxWidth()
                            .background(TetherTheme.colors.surfaceSunken)
                            .padding(TetherSpace.space3),
                )
            }
            turnError?.let { message ->
                InlineBanner(
                    text = message,
                    tone = BannerTone.Danger,
                    modifier = Modifier.padding(TetherSpace.space4),
                )
            }
            StatusLine(status, usage, onModelClick, onModeClick)
            composer?.invoke()
        }
    }
}

/**
 * Keeps the newest message in view while the reader is at the bottom. The flag changes only when
 * the scroll position changes, so a streamed reply that grows below the viewport does not count as
 * the reader leaving the bottom.
 */
@Composable
private fun FollowTail(
    listState: LazyListState,
    messages: List<MessageView>,
) {
    var followTail by remember { mutableStateOf(true) }
    LaunchedEffect(listState) {
        snapshotFlow { listState.firstVisibleItemIndex to listState.firstVisibleItemScrollOffset }
            .collect { followTail = listState.isAtBottom() }
    }
    LaunchedEffect(messages) {
        val last = messages.lastIndex
        if (last >= 0 && shouldFollow(followTail, hasNewContent = true)) {
            // A large offset clamps to the list end, so a growing reply stays in view.
            listState.scrollToItem(last, Int.MAX_VALUE)
        }
    }
}

/** True when the last item is fully inside the viewport. */
internal fun LazyListState.isAtBottom(): Boolean {
    val info = layoutInfo
    val last = info.visibleItemsInfo.lastOrNull() ?: return true
    return last.index == info.totalItemsCount - 1 &&
        last.offset + last.size <= info.viewportEndOffset
}

/** Scroll to the newest content only when the reader was already at the bottom. */
internal fun shouldFollow(
    wasAtBottom: Boolean,
    hasNewContent: Boolean,
): Boolean = wasAtBottom && hasNewContent

/** Events that end a turn, including a stopped one. Each one clears the running state. */
internal fun CoreEventView.endsTurn(): Boolean =
    when (this) {
        is CoreEventView.TurnFinished,
        is CoreEventView.TurnFailed,
        is CoreEventView.TurnInterrupted,
        -> true
        else -> false
    }

private fun CoreEventView.proposedToolOr(current: Pair<String, String>?): Pair<String, String>? =
    if (this is CoreEventView.ToolProposed) tool to target else current

/** The error a failed turn reports, or [fallback] when the core sent no text. Else null. */
private fun CoreEventView.turnFailureOr(fallback: String): String? =
    if (this is CoreEventView.TurnFailed) error.message.ifBlank { fallback } else null

@Composable
private fun TranscriptMessage(
    message: MessageView,
    modifier: Modifier = Modifier,
) {
    val label =
        when (message.role) {
            RoleView.USER -> R.string.transcript_you
            RoleView.ASSISTANT -> R.string.transcript_agent
            RoleView.TOOL -> R.string.transcript_tool
            RoleView.SYSTEM_NOTE -> R.string.transcript_system
        }
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    ) {
        Text(
            stringResource(label),
            style = MaterialTheme.typography.labelMedium,
            color = TetherTheme.colors.text2,
        )
        message.parts.forEach { part ->
            when (part) {
                is MessagePartView.Text -> MarkdownBlocks(part.text)
                is MessagePartView.Thinking -> Text(part.text, color = TetherTheme.colors.text2)
                is MessagePartView.CompactionSummary ->
                    Text(part.text, color = TetherTheme.colors.text2)
                else ->
                    Text(
                        stringResource(R.string.transcript_attachment),
                        color = TetherTheme.colors.text2,
                    )
            }
        }
    }
}
