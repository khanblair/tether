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
import app.tether.providers.providerNameRes
import app.tether.session.agent.AgentScreenUi
import app.tether.session.agent.PartSegment
import app.tether.session.agent.ToolCallRows
import app.tether.session.agent.ToolCardState
import app.tether.session.agent.agentLiveItems
import app.tether.session.agent.liveSection
import app.tether.session.agent.segmentParts
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
) {
    onStart()
    launch {
        try {
            withContext(Dispatchers.IO) { connection.send(text) }
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
) {
    val scope = rememberCoroutineScope()
    Composer(
        modelId = modelId,
        running = running,
        onSend = { text -> scope.sendTurn(connection, text, onSendStart, onSendFailed) },
        onStop = { scope.launch(Dispatchers.IO) { connection.stop() } },
        sendBlocked = sendBlocked,
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
) {
    val footer = turn.hasFooter
    val rowModifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth()
    val hasLive = agent?.liveSection()?.isNotEmpty() == true
    if (messages.isEmpty() && !footer && !hasLive) {
        val emptyLabel = if (failed) R.string.transcript_load_failed else R.string.transcript_empty
        Text(
            text = stringResource(emptyLabel),
            color = TetherTheme.colors.text2,
            modifier = Modifier.padding(TetherSpace.space4),
        )
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
 * One persisted message. A long press opens its actions when the caller passes [onLongPress].
 * The press carries a label, so TalkBack offers it as a named action.
 */
@Composable
internal fun TranscriptMessage(
    message: MessageView,
    modifier: Modifier = Modifier,
    onLongPress: (() -> Unit)? = null,
    tools: ToolCardState? = null,
) {
    val label =
        when (message.role) {
            RoleView.USER -> R.string.transcript_you
            RoleView.ASSISTANT -> R.string.transcript_agent
            RoleView.TOOL -> R.string.transcript_tool
            RoleView.SYSTEM_NOTE -> R.string.transcript_system
        }
    val longPressLabel = stringResource(R.string.states_message_actions)
    val pressModifier =
        if (onLongPress == null) {
            Modifier
        } else {
            Modifier.combinedClickable(
                onClick = {},
                onLongClickLabel = longPressLabel,
                onLongClick = onLongPress,
            )
        }
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = modifier.then(pressModifier).padding(horizontal = TetherSpace.space4),
    ) {
        Text(
            stringResource(label),
            style = MaterialTheme.typography.labelMedium,
            color = TetherTheme.colors.text2,
        )
        val toolState = tools ?: ToolCardState.Empty
        segmentParts(message.parts, toolState.persisted).forEach { segment ->
            when (segment) {
                is PartSegment.Calls -> ToolCallRows(segment.calls, toolState)
                is PartSegment.Part -> MessagePartBody(segment.part)
            }
        }
    }
}

/** One message part that is not a tool call. */
@Composable
private fun MessagePartBody(part: MessagePartView) {
    when (part) {
        is MessagePartView.Text -> MarkdownBlocks(part.text)
        is MessagePartView.Thinking -> Text(part.text, color = TetherTheme.colors.text2)
        is MessagePartView.CompactionSummary -> Text(part.text, color = TetherTheme.colors.text2)
        else ->
            Text(
                stringResource(R.string.transcript_attachment),
                color = TetherTheme.colors.text2,
            )
    }
}
