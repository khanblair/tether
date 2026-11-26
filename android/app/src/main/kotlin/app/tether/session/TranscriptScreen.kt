package app.tether.session

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
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
import app.tether.session.agent.toolCallsFromMessages
import app.tether.session.compose.CommandScreen
import app.tether.session.compose.commandHooksOf
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

    LaunchedEffect(agent) { agent?.hydrate() }
    SnapshotReload(
        connection = connection,
        reloads = sheetControl.reloads,
        onLoaded = { snapshot = it },
        onFailed = { failed = true },
    )
    CoreEvents(
        connection = connection,
        onEvent = { event ->
            agent?.onEvent(event)
            turn = turn.onEvent(event, System.currentTimeMillis())
        },
        onLoaded = { snapshot = it },
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
        agent = agentUi,
        onSendPlanEdit =
            connection?.let { active ->
                { text: String -> sendPlanEdit(active, text) }
            },
        onOpenBudgets = onOpenBudgets ?: {},
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

/** Opens a command screen for this session. Without sheets, the action is missing. */
private fun screenOpener(
    sheets: SessionSheetEnv?,
    target: CommandScreen,
): (() -> Unit)? = sheets?.let { env -> { env.openScreen(target, "") } }

/** A sheet opener for a session with sheets. Without sheets, the control has no action. */
private fun sheetOpener(
    sheets: SessionSheetEnv?,
    control: SessionSheetControl,
    kind: SessionSheetKind,
): (() -> Unit)? = sheets?.let { { control.openSheet(kind) } }

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
    composer: (@Composable () -> Unit)? = null,
    agent: AgentScreenUi? = null,
    onSendPlanEdit: ((String) -> Unit)? = null,
    onOpenBudgets: () -> Unit = {},
) {
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    val reducedMotion = rememberReducedMotion()
    val visible = remember(messages) { messages.filterNot { it.isToolResultOnly() } }
    val live = agent?.liveSection().orEmpty()
    val footerRows = if (turn.hasFooter) 1 else 0
    val unseen =
        FollowTail(listState, visible, footerRows + live.size, liveKey = agent?.state?.calls)
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
    val tools = ToolCardState(persisted, expanded.ids, expanded::toggle)
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
                )
                JumpToLatest(
                    count = unseen,
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
            composer?.invoke()
        }
    }
}

/**
 * Keeps the newest item in view while the reader is at the bottom. [trailing] counts the items
 * after the messages: the live section and the turn footer. [liveKey] changes with each live
 * update, so a streaming tool card keeps the tail in view. The flag changes only when the scroll
 * position changes, so a streamed reply that grows below the viewport does not count as the
 * reader leaving the bottom. Returns the rows that arrived since the reader left the bottom.
 */
@Composable
private fun FollowTail(
    listState: LazyListState,
    messages: List<MessageView>,
    trailing: Int,
    liveKey: Any?,
): Int {
    var followTail by remember { mutableStateOf(true) }
    var itemsWhenLeft by remember { mutableIntStateOf(0) }
    var itemsNow by remember { mutableIntStateOf(0) }
    LaunchedEffect(listState) {
        snapshotFlow { listState.firstVisibleItemIndex to listState.firstVisibleItemScrollOffset }
            .collect {
                val atBottom = listState.isAtBottom()
                if (followTail && !atBottom) itemsWhenLeft = listState.layoutInfo.totalItemsCount
                followTail = atBottom
            }
    }
    LaunchedEffect(listState) {
        snapshotFlow { listState.layoutInfo.totalItemsCount }.collect { itemsNow = it }
    }
    LaunchedEffect(messages, trailing, liveKey) {
        val lastItem = messages.size + trailing - 1
        if (lastItem >= 0 && shouldFollow(followTail, hasNewContent = true)) {
            // A large offset clamps to the list end, so a growing reply stays in view.
            listState.scrollToItem(lastItem, Int.MAX_VALUE)
        }
    }
    return newItemCount(itemsWhenLeft, itemsNow, followTail)
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
