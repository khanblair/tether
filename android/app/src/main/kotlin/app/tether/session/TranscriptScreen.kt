package app.tether.session

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.MessageView
import app.tether.core.generated.ProviderView
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.tokens.TetherSpace
import app.tether.git.rememberReducedMotion
import app.tether.messages.AppMessages
import app.tether.session.agent.AgentApprovalSlot
import app.tether.session.agent.AgentScreenUi
import app.tether.session.agent.AgentTopBlock
import app.tether.session.agent.ExpandedCards
import app.tether.session.agent.ToolCardState
import app.tether.session.agent.expandedCardsSaver
import app.tether.session.agent.isToolResultOnly
import app.tether.session.agent.liveSection
import app.tether.session.agent.rememberAgentLoop
import app.tether.session.agent.rememberAgentScreen
import app.tether.session.agent.savedPreviewPaths
import app.tether.session.agent.toolCallsFromMessages
import app.tether.session.compose.CommandScreen
import app.tether.session.compose.commandHooksOf
import app.tether.session.servers.ServerBar
import app.tether.session.servers.ServerPreviewTarget
import app.tether.session.servers.ServerState
import app.tether.session.servers.ServerStateHolder
import app.tether.session.servers.serverPreviewAction
import app.tether.session.servers.serverStopAction
import kotlinx.coroutines.launch

/** Persisted messages are read off the main thread when the session route opens. */
@Composable
fun TranscriptScreen(
    connection: SessionConnection?,
    onBack: () -> Unit,
    sheets: SessionSheetEnv? = null,
    onOpenBudgets: (() -> Unit)? = null,
) {
    var snapshot by remember(connection) { mutableStateOf<TranscriptSnapshot?>(null) }
    var failed by remember(connection) { mutableStateOf(false) }
    var turn by remember(connection) { mutableStateOf(TurnState()) }
    var actionsFor by remember(connection) { mutableStateOf<MessageView?>(null) }
    val sheetControl = remember(connection) { SessionSheetControl() }
    val scope = rememberCoroutineScope()
    val offline = rememberOffline()
    val messages = snapshot?.messages.orEmpty()
    val agent = rememberAgentLoop(connection)
    val agentUi = rememberAgentScreen(agent)
    val project = rememberProjectSummary(sheets?.projects, connection?.projectId)
    val serverState = remember(connection) { ServerStateHolder() }
    val servers by serverState.state.collectAsStateWithLifecycle()
    var serverPreview by remember(connection) { mutableStateOf<ServerPreviewTarget?>(null) }
    val stopFailed = stringResource(R.string.server_stop_failed)
    val previewReady = stringResource(R.string.server_preview_ready)
    val openLabel = stringResource(R.string.server_open)
    val stoppedFormat = stringResource(R.string.server_stopped_code)
    val openServerPreview = serverPreviewAction(sheets)
    val stopServer =
        serverStopAction(scope, connection) { sheets?.messages?.show(stopFailed) }
    val onSnapshotLoaded: (TranscriptSnapshot) -> Unit = {
        serverState.loadOnce(it.processes)
        snapshot = it
    }

    /** A turn starts: the last turn's cards clear, and the send shows as running. */
    fun startTurn() {
        agent?.onNewTurn()
        turn = turn.onSendStarted()
    }

    fun retryLast() {
        val active = connection ?: return
        if (agentUi?.approvalsWaiting == true) return
        scope.resendLastMessage(
            active,
            messages,
            onStart = { startTurn() },
            onFailed = { turn = turn.onSendFailed() },
        )
    }

    /** Sends the plan back as a message. The plan card closes first. */
    fun sendPlanEdit(
        active: SessionConnection,
        text: String,
    ) {
        if (agentUi?.approvalsWaiting == true) return
        agent?.onPlanSent()
        scope.sendTurn(active, text, onStart = { startTurn() }, onFailed = {
            turn =
                turn.onSendFailed()
        })
    }
    val chat =
        rememberChatActions(
            connection = connection,
            messages = messages,
            busy = sessionBusy(turn, agentUi),
            onStart = { startTurn() },
            onFailed = { turn = turn.onSendFailed() },
        )
    LaunchedEffect(agent) { agent?.hydrate() }
    SnapshotReload(
        connection = connection,
        reloads = sheetControl.reloads,
        onLoaded = onSnapshotLoaded,
        onFailed = { failed = true },
    )
    CoreEvents(
        connection = connection,
        onEvent = { event ->
            agent?.onEvent(event)
            turn = turn.onEvent(event, System.currentTimeMillis())
            handleServerEvent(event, serverState, sheets, stoppedFormat) { target ->
                serverPreview = target
                sheets?.messages?.show(previewReady, openLabel) { openServerPreview(target) }
            }
        },
        onLoaded = onSnapshotLoaded,
        onFailed = { failed = true },
    )
    TranscriptContent(
        messages = messages,
        failed = failed,
        title = topBarTitle(snapshot?.details?.title),
        subtitle =
            sessionSubtitle(
                projectName = project?.name,
                branch = project?.branch.orEmpty(),
                format = stringResource(R.string.session_subtitle_branch),
            ),
        status = snapshot?.let { it.status.shownWith(it.bypass) },
        modelName = snapshot?.modelName,
        usage = snapshot?.usage,
        turn = turn,
        provider = snapshot?.details?.provider,
        offline = offline,
        onBack = onBack,
        onOpenDrawer = sheets?.openDrawer,
        onReviewChanges = screenOpener(sheets, CommandScreen.Diff),
        onOpenCost = screenOpener(sheets, CommandScreen.Cost),
        onModelClick = sheetOpener(sheets, sheetControl, SessionSheetKind.Model),
        onModeClick = sheetOpener(sheets, sheetControl, SessionSheetKind.Mode),
        onMore = sheetOpener(sheets, sheetControl, SessionSheetKind.Menu),
        onRetry = if (connection != null && lastUserText(messages) != null) ::retryLast else null,
        onOpenProviders = sheets?.openProviders,
        onDismissStopped = { turn = turn.onDismissStopped() },
        onLongPress = { actionsFor = it },
        onEditMessage = chat.onEditMessage,
        onRegenerate = chat.onRegenerate,
        toasts = sheets?.messages,
        onPreviewFile = sheets?.openPreview,
        onPreviewHtml = sheets?.openInlinePreview,
        agent = agentUi,
        onSendPlanEdit =
            connection?.let { active ->
                { text: String -> sendPlanEdit(active, text) }
            },
        onOpenBudgets = onOpenBudgets ?: {},
        servers = servers,
        serverPreview = serverPreview,
        onPreviewServer = openServerPreview,
        onStopServer = stopServer,
        composer =
            connection?.let { active ->
                {
                    ComposerControls(
                        connection = active,
                        modelId = snapshot?.modelName.orEmpty(),
                        running = turn.running,
                        onSendStart = { startTurn() },
                        onSendFailed = { turn = turn.onSendFailed() },
                        sendBlocked = agentUi?.approvalsWaiting == true,
                        files = sheets?.files,
                        messages = sheets?.messages,
                        commands = sheets?.let { commandHooksOf(it, sheetControl) },
                        mode = composerMode(snapshot),
                        supportsImages = snapshot?.supportsImages == true,
                        onOpenMode = sheetOpener(sheets, sheetControl, SessionSheetKind.Mode),
                        onOpenModel = sheetOpener(sheets, sheetControl, SessionSheetKind.Model),
                    )
                }
            },
    )
    MessageSheetSlot(
        message = actionsFor,
        toasts = sheets?.messages,
        onDone = { actionsFor = null },
    )
    ChatEditSlot(chat)
    SessionSheetsSlot(
        connection = connection,
        sheets = sheets,
        control = sheetControl,
        isRunning = { turn.running },
    )
}

/** The top bar title: the session's title, or the route title while the snapshot is loading. */
@Composable
private fun topBarTitle(title: String?): String =
    if (title == null) stringResource(R.string.route_session_title) else sessionTitleText(title)

/** The sheet for the message a long press chose. It is hidden while [message] is null. */
@Composable
private fun MessageSheetSlot(
    message: MessageView?,
    toasts: AppMessages?,
    onDone: () -> Unit,
) {
    val copy = rememberCopyMessage(toasts)
    message?.let { chosen ->
        MessageActionsSheet(
            onCopy = {
                copy(chosen)
                onDone()
            },
            onDismiss = onDone,
        )
    }
}

/** Lazy items use persisted IDs so rows keep their identity as text arrives. */
@Composable
fun TranscriptContent(
    messages: List<MessageView>,
    failed: Boolean,
    title: String,
    subtitle: String? = null,
    status: SessionStatus? = null,
    modelName: String? = null,
    usage: SessionUsage? = null,
    turn: TurnState = TurnState(),
    provider: ProviderView? = null,
    offline: Boolean = false,
    onBack: () -> Unit,
    onModelClick: (() -> Unit)? = null,
    onModeClick: (() -> Unit)? = null,
    onMore: (() -> Unit)? = null,
    onOpenDrawer: (() -> Unit)? = null,
    onReviewChanges: (() -> Unit)? = null,
    onOpenCost: (() -> Unit)? = null,
    onRetry: (() -> Unit)? = null,
    onOpenProviders: (() -> Unit)? = null,
    onDismissStopped: () -> Unit = {},
    onLongPress: ((MessageView) -> Unit)? = null,
    onEditMessage: ((MessageView) -> Unit)? = null,
    onRegenerate: (() -> Unit)? = null,
    toasts: AppMessages? = null,
    onPreviewFile: ((String) -> Unit)? = null,
    onPreviewHtml: ((String) -> Unit)? = null,
    composer: (@Composable () -> Unit)? = null,
    agent: AgentScreenUi? = null,
    onSendPlanEdit: ((String) -> Unit)? = null,
    onOpenBudgets: () -> Unit = {},
    servers: ServerState = ServerState(),
    serverPreview: ServerPreviewTarget? = null,
    onPreviewServer: (ServerPreviewTarget) -> Unit = {},
    onStopServer: (String) -> Unit = {},
) {
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val reducedMotion = rememberReducedMotion()
    val visible = remember(messages) { messages.filterNot { it.isToolResultOnly() } }
    val live = agent?.liveSection().orEmpty()
    val footerRows = if (turn.hasFooter) 1 else 0
    val jump =
        followTail(listState, visible, footerRows + live.size, liveKey = agent?.state?.calls)
    val expanded = rememberSaveable(saver = expandedCardsSaver) { ExpandedCards() }
    val liveIds =
        agent
            ?.state
            ?.calls
            .orEmpty()
            .map { it.id }
            .toSet()
    val persisted =
        remember(messages, liveIds) {
            toolCallsFromMessages(messages).filterKeys { id -> id !in liveIds }
        }
    val previewPaths = remember(messages, liveIds) { savedPreviewPaths(messages) }
    val tools =
        ToolCardState(persisted, expanded.ids, expanded::toggle, onPreviewFile, previewPaths)
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            SessionTopBar(
                title = title,
                subtitle = subtitle,
                onBack = onBack,
                onOpenDrawer = onOpenDrawer,
                onReviewChanges = onReviewChanges,
                onMore = onMore,
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            agent?.let { AgentTopBlock(ui = it, onOpenBudgets = onOpenBudgets) }
            Box(modifier = Modifier.weight(1f)) {
                TranscriptList(
                    messages = visible,
                    failed = failed,
                    turn = turn,
                    provider = provider,
                    listState = listState,
                    onRetry = onRetry,
                    onOpenProviders = onOpenProviders,
                    onDismissStopped = onDismissStopped,
                    onLongPress = onLongPress,
                    agent = agent,
                    tools = tools,
                    onSendPlanEdit = onSendPlanEdit ?: {},
                    onEditMessage = onEditMessage,
                    onRegenerate = onRegenerate,
                    toasts = toasts,
                    onPreviewHtml = onPreviewHtml,
                )
                JumpToLatest(
                    pill = jump,
                    onClick = { scope.launch { scrollToLatest(listState, reducedMotion) } },
                    modifier =
                        Modifier
                            .align(Alignment.BottomEnd)
                            .padding(
                                horizontal = TetherSpace.space4,
                                vertical = TetherSpace.space3,
                            ),
                )
            }
            turn.proposedTool?.let { (tool, target) -> ProposedToolBar(tool, target) }
            StatusLine(status, usage, modelName, onModelClick, onModeClick, onOpenCost)
            if (offline) {
                InlineBanner(
                    text = stringResource(R.string.states_offline),
                    tone = BannerTone.Warning,
                    modifier = Modifier.padding(horizontal = TetherSpace.space4),
                )
            }
            agent?.let {
                AgentApprovalSlot(
                    ui = it,
                    modifier = Modifier.padding(top = TetherSpace.space2),
                )
            }
            ServerBar(
                state = servers,
                preview = serverPreview,
                onPreview = onPreviewServer,
                onStop = onStopServer,
                modifier = Modifier.padding(horizontal = TetherSpace.space3),
            )
            composer?.invoke()
        }
    }
}

/** Events that end a turn, including a stopped one. Each one clears the running state. */
internal fun CoreEventView.endsTurn(): Boolean =
    when (this) {
        is CoreEventView.TurnFinished,
        is CoreEventView.TurnFailed,
        is CoreEventView.TurnInterrupted,
        -> true
        else -> false
    }
