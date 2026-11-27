package app.tether.session

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
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
import app.tether.session.agent.savedPreviewPaths
import app.tether.session.agent.toolCallsFromMessages
import app.tether.session.servers.ServerBar
import app.tether.session.servers.ServerPreviewTarget
import app.tether.session.servers.ServerState
import kotlinx.coroutines.launch

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
