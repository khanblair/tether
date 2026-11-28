package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.ProviderView
import app.tether.core.generated.RoleView
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.files.FilesGateway
import app.tether.messages.AppMessages
import app.tether.providers.providerNameRes
import app.tether.session.agent.AgentScreenUi
import app.tether.session.agent.PartSegment
import app.tether.session.agent.ToolCallRows
import app.tether.session.agent.ToolCardState
import app.tether.session.agent.agentLiveItems
import app.tether.session.agent.liveSection
import app.tether.session.agent.segmentParts
import app.tether.session.compose.CommandHooks
import app.tether.session.compose.ComposeAssistDeps
import app.tether.session.compose.SkippedMention
import app.tether.session.compose.rememberCommandSend
import app.tether.session.compose.skippedMentionsLine
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val TURN_FOOTER_KEY = "turn-footer"

/**
 * Sends [text] as a new turn on the IO dispatcher. [onStart] runs first, so the screen shows the
 * turn as running at once. [onFailed] runs when the core refuses the send.
 */
internal fun CoroutineScope.sendTurn(
    connection: SessionConnection,
    text: String,
    onStart: () -> Unit,
    onFailed: () -> Unit,
    onSkipped: (List<SkippedMention>) -> Unit = {},
) {
    onStart()
    launch {
        try {
            val skipped = withContext(Dispatchers.IO) { connection.send(text) }
            if (skipped.isNotEmpty()) onSkipped(skipped)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            onFailed()
        }
    }
}

/** Sends the newest user message again as a new turn. Does nothing when there is none. */
internal fun CoroutineScope.resendLastMessage(
    connection: SessionConnection,
    messages: List<MessageView>,
    onStart: () -> Unit,
    onFailed: () -> Unit,
) {
    val text = lastUserText(messages) ?: return
    sendTurn(connection, text, onStart, onFailed)
}

@Composable
internal fun ComposerControls(
    connection: SessionConnection,
    modelId: String,
    running: Boolean,
    onSendStart: () -> Unit,
    onSendFailed: () -> Unit,
    sendBlocked: Boolean = false,
    files: FilesGateway? = null,
    messages: AppMessages? = null,
    commands: CommandHooks? = null,
    mode: SessionMode = SessionMode.ACCEPT_EDITS,
    supportsImages: Boolean = false,
    onOpenMode: (() -> Unit)? = null,
    onOpenModel: (() -> Unit)? = null,
) {
    val scope = rememberCoroutineScope()
    val skippedText = stringResource(R.string.compose_skipped_files)
    val assist =
        remember(connection, files) {
            files?.let { ComposeAssistDeps(connection.commands, it, connection.projectId) }
        }
    val sendPlain: (String) -> Unit = { text ->
        scope.sendTurn(connection, text, onSendStart, onSendFailed) { skipped ->
            messages?.show(skippedText.format(skippedMentionsLine(skipped)))
        }
    }
    Composer(
        modelLabel = modelId,
        mode = mode,
        running = running,
        onSend = rememberCommandSend(connection, commands, messages, sendPlain),
        onStop = { scope.launch(Dispatchers.IO) { connection.stop() } },
        onOpenModel = onOpenModel,
        onOpenMode = onOpenMode,
        supportsImages = supportsImages,
        sendBlocked = sendBlocked,
        assist = assist,
    )
}

/**
 * The transcript rows, or the empty line when there is nothing to show. [messages] are the rows to
 * draw. The live section of [agent] follows them, then the turn footer.
 */
@Composable
internal fun TranscriptList(
    messages: List<MessageView>,
    failed: Boolean,
    turn: TurnState,
    provider: ProviderView?,
    listState: LazyListState,
    onRetry: (() -> Unit)?,
    onOpenProviders: (() -> Unit)?,
    onDismissStopped: () -> Unit,
    onLongPress: ((MessageView) -> Unit)?,
    agent: AgentScreenUi? = null,
    tools: ToolCardState = ToolCardState.Empty,
    onSendPlanEdit: (String) -> Unit = {},
    onEditMessage: ((MessageView) -> Unit)? = null,
    onRegenerate: (() -> Unit)? = null,
    toasts: AppMessages? = null,
    onPreviewHtml: ((String) -> Unit)? = null,
    emptyState: (@Composable () -> Unit)? = null,
) {
    val footer = turn.hasFooter
    val rowModifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth()
    val hasLive = agent?.liveSection()?.isNotEmpty() == true
    val anchors = remember(messages, turn.running) { turnAnchors(messages, turn.running) }
    val copy = rememberCopyMessage(toasts)
    val liveId = if (turn.running) messages.lastOrNull()?.id else null
    if (messages.isEmpty() && !footer && !hasLive) {
        EmptyTranscript(failed, emptyState)
        return
    }
    LazyColumn(
        state = listState,
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxSize(),
    ) {
        items(messages, key = MessageView::id) { message ->
            TranscriptMessage(
                message,
                rowModifier,
                onLongPress = longPressFor(message, onLongPress),
                tools = tools,
                anchor = anchors[message.id],
                live = message.id == liveId,
                onCopy = copy,
                onEdit = onEditMessage,
                onRegenerate = onRegenerate,
                onPreviewHtml = if (message.id == liveId) null else onPreviewHtml,
            )
        }
        if (agent != null) agentLiveItems(agent, tools, rowModifier, onSendPlanEdit)
        if (footer) {
            item(key = TURN_FOOTER_KEY) {
                TurnFooter(
                    turn = turn,
                    providerName = providerLabel(provider),
                    onRetry = onRetry,
                    onOpenProviders = onOpenProviders,
                    onDismissStopped = onDismissStopped,
                    modifier = rowModifier,
                )
            }
        }
    }
}

/** The tool a running turn proposed, above the status line. */
@Composable
internal fun ProposedToolBar(
    tool: String,
    target: String,
) {
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

/** The long press for a message with text, or null when there is nothing to act on. */
private fun longPressFor(
    message: MessageView,
    onLongPress: ((MessageView) -> Unit)?,
): (() -> Unit)? =
    if (onLongPress == null || plainText(message).isBlank()) {
        null
    } else {
        ({ onLongPress(message) })
    }

@Composable
private fun providerLabel(provider: ProviderView?): String =
    provider?.let { stringResource(providerNameRes(it)) }
        ?: stringResource(R.string.states_provider_unknown)

/**
 * One persisted message. A user message is a bubble on the right. Other messages are plain text
 * on the left, with their tool cards and reasoning blocks. A long press opens the message actions
 * when the caller passes [onLongPress]. The press carries a label, so TalkBack offers it by name.
 */
@Composable
internal fun TranscriptMessage(
    message: MessageView,
    modifier: Modifier = Modifier,
    onLongPress: (() -> Unit)? = null,
    tools: ToolCardState? = null,
    anchor: TurnAnchor? = null,
    live: Boolean = false,
    onCopy: ((MessageView) -> Unit)? = null,
    onEdit: ((MessageView) -> Unit)? = null,
    onRegenerate: (() -> Unit)? = null,
    onPreviewHtml: ((String) -> Unit)? = null,
) {
    if (message.role == RoleView.USER) {
        UserBubble(
            message = message,
            modifier = modifier,
            onLongPress = onLongPress,
            onCopy = onCopy?.let { copyTo -> { copyTo(message) } },
            onEdit = onEdit?.let { editTo -> { editTo(message) } },
        )
        return
    }
    val pressModifier =
        if (onLongPress == null) {
            Modifier
        } else {
            Modifier.combinedClickable(
                onClick = {},
                onLongClickLabel = stringResource(R.string.states_message_actions),
                onLongClick = onLongPress,
            )
        }
    val toolState = tools ?: ToolCardState.Empty
    val pressed = modifier.then(pressModifier)
    if (message.role == RoleView.SYSTEM_NOTE) {
        SystemNote(message, pressed)
    } else {
        AgentMessage(message, pressed, toolState, live, anchor, onCopy, onRegenerate, onPreviewHtml)
    }
}

/** A system note: one quiet line, without a label. */
@Composable
private fun SystemNote(
    message: MessageView,
    modifier: Modifier,
) {
    Text(
        text = plainText(message),
        style = MaterialTheme.typography.labelMedium,
        color = TetherTheme.colors.text2,
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    )
}

/**
 * An assistant or tool message: its text as markdown, its reasoning as one block, its tool calls
 * as cards. The last assistant message of a turn also carries the turn's actions.
 */
@Composable
private fun AgentMessage(
    message: MessageView,
    modifier: Modifier,
    tools: ToolCardState,
    live: Boolean,
    anchor: TurnAnchor?,
    onCopy: ((MessageView) -> Unit)?,
    onRegenerate: (() -> Unit)?,
    onPreviewHtml: ((String) -> Unit)?,
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    ) {
        val runs = thinkingRuns(segmentParts(message.parts, tools.persisted))
        runs.forEachIndexed { index, run ->
            if (isThinking(run.first())) {
                ThinkingBlock(
                    stateKey = "${message.id}#$index",
                    text = thinkingText(run),
                    live = live,
                )
            } else {
                run.forEach { segment -> AgentSegment(segment, tools, onPreviewHtml) }
            }
        }
        if (anchor != null) TurnActions(message, anchor, onCopy, onRegenerate)
    }
}

/** One segment that is not reasoning: a run of tool cards, or one message part. */
@Composable
private fun AgentSegment(
    segment: PartSegment,
    tools: ToolCardState,
    onPreviewHtml: ((String) -> Unit)?,
) {
    when (segment) {
        is PartSegment.Calls -> ToolCallRows(segment.calls, tools)
        is PartSegment.Part -> MessagePartBody(segment.part, onPreviewHtml)
    }
}

/**
 * The Copy and Regenerate icons under the last reply of a turn. Copy takes the turn's reply text,
 * so it leaves out thinking and tool calls. Regenerate shows only on the newest turn.
 */
@Composable
private fun TurnActions(
    message: MessageView,
    anchor: TurnAnchor,
    onCopy: ((MessageView) -> Unit)?,
    onRegenerate: (() -> Unit)?,
) {
    val copy: (() -> Unit)? =
        if (onCopy == null || anchor.replyText.isBlank()) {
            null
        } else {
            replyCopy(message, anchor.replyText, onCopy)
        }
    val regenerate = if (anchor.newest) onRegenerate else null
    if (copy != null || regenerate != null) {
        MessageActionRow(
            copyLabel = stringResource(R.string.chat_copy_reply),
            onCopy = copy,
            onRegenerate = regenerate,
        )
    }
}

/** A copy action for [text] as the message's only part. */
private fun replyCopy(
    message: MessageView,
    text: String,
    onCopy: (MessageView) -> Unit,
): () -> Unit {
    val reply = message.copy(parts = listOf(MessagePartView.Text(text)))
    return { onCopy(reply) }
}

/** One message part that is not a tool call. */
@Composable
internal fun MessagePartBody(
    part: MessagePartView,
    onPreviewHtml: ((String) -> Unit)? = null,
) {
    when (part) {
        is MessagePartView.Text -> MarkdownBlocks(part.text, onPreviewHtml)
        is MessagePartView.Thinking -> Text(part.text, color = TetherTheme.colors.text2)
        is MessagePartView.CompactionSummary -> Text(part.text, color = TetherTheme.colors.text2)
        else ->
            Text(
                stringResource(R.string.transcript_attachment),
                color = TetherTheme.colors.text2,
            )
    }
}
