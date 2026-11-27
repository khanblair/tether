package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.MessageView
import app.tether.messages.AppMessages
import app.tether.session.agent.rememberAgentLoop
import app.tether.session.agent.rememberAgentScreen
import app.tether.session.compose.CommandScreen
import app.tether.session.compose.commandHooksOf
import app.tether.session.compose.skippedMentionsLine
import app.tether.session.servers.ServerPreviewTarget
import app.tether.session.servers.ServerStateHolder
import app.tether.session.servers.serverPreviewAction
import app.tether.session.servers.serverStopAction

/** Persisted messages are read off the main thread when the session route opens. */
@Composable
fun TranscriptScreen(
    connection: SessionConnection?,
    initialMessage: String? = null,
    onInitialMessageStarted: () -> Unit = {},
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
    val skippedText = stringResource(R.string.compose_skipped_files)
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
    SendInitialMessage(
        connection = connection,
        initialMessage = initialMessage,
        send = { active, text ->
            scope.sendTurn(
                connection = active,
                text = text,
                onStart = { startTurn() },
                onFailed = { turn = turn.onSendFailed() },
                onSkipped = { skipped ->
                    sheets?.messages?.show(skippedText.format(skippedMentionsLine(skipped)))
                },
            )
        },
        onStarted = onInitialMessageStarted,
    )
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

/** Events that end a turn, including a stopped one. Each one clears the running state. */
internal fun CoreEventView.endsTurn(): Boolean =
    when (this) {
        is CoreEventView.TurnFinished,
        is CoreEventView.TurnFailed,
        is CoreEventView.TurnInterrupted,
        -> true
        else -> false
    }
