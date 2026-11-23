package app.tether.session.agent

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.tether.design.tokens.TetherSpace
import app.tether.session.approval.AlwaysAllowMenu
import app.tether.session.approval.ApprovalBar
import app.tether.session.approval.ApprovalScreenState
import app.tether.session.approval.ApprovalSheet
import app.tether.session.approval.ApprovalViewModel
import app.tether.session.approval.DenyReasonSheet
import app.tether.session.approval.alwaysAllowRule
import app.tether.session.cards.AskUserCard
import app.tether.session.cards.ChecklistPin
import app.tether.session.cards.PlanCard
import app.tether.session.cards.checklistShown
import app.tether.session.cards.currentChecklistItem
import app.tether.session.notices.BudgetReachedBanner
import app.tether.session.notices.BudgetWarningBanner
import app.tether.session.notices.InterruptedBlock
import app.tether.session.notices.NetworkBanner
import app.tether.session.notices.NoticeAction
import app.tether.session.tools.QualityReportCard

/**
 * The notices and the checklist above the transcript: the network pause, the budget warnings and
 * stops, the interrupted turn, and the checklist pin while it is not dismissed.
 */
@Composable
internal fun AgentTopBlock(
    ui: AgentScreenUi,
    onOpenBudgets: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val notices = ui.notices
    val vm = ui.loop.notices
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        if (notices.networkPaused) NetworkBanner()
        notices.warnings.forEach { BudgetWarningBanner(warning = it) }
        notices.reached.forEach { reached ->
            BudgetReachedBanner(
                reached = reached,
                busy = notices.running == NoticeAction.Continue,
                failed = notices.failed == NoticeAction.Continue,
                onContinue = { vm.onContinue() },
                onOpenBudgets = onOpenBudgets,
            )
        }
        notices.interruption?.let { interruption ->
            InterruptedBlock(
                interruption = interruption,
                running = notices.running,
                failed = notices.failed,
                onResume = { vm.onResume() },
                onDiscard = { vm.onDiscard() },
            )
        }
        ChecklistSlot(ui)
    }
}

/** The checklist pin, when there are items and the user has not dismissed a finished list. */
@Composable
private fun ChecklistSlot(ui: AgentScreenUi) {
    val todos = ui.cards.todos
    if (!checklistShown(todos, ui.cards.ui.checklistDismissed)) return
    val cards = ui.loop.cards
    ChecklistPin(
        items = todos,
        current = currentChecklistItem(todos),
        expanded = ui.cards.ui.checklistExpanded,
        onToggleExpanded = { cards.toggleChecklist() },
        onDismiss = { cards.dismissChecklist() },
        modifier = Modifier.padding(horizontal = TetherSpace.space4),
    )
}

/**
 * The live section of the current turn, as lazy items after the last message. [rowModifier] is the
 * width the transcript rows use. [tools] gives the expansion state of the tool cards.
 */
internal fun LazyListScope.agentLiveItems(
    ui: AgentScreenUi,
    tools: ToolCardState,
    rowModifier: Modifier,
    onSendPlanEdit: (String) -> Unit,
) {
    items(ui.liveSection(), key = LiveItem::key) { item ->
        LiveItemContent(item, ui, tools, rowModifier, onSendPlanEdit)
    }
}

@Composable
private fun LiveItemContent(
    item: LiveItem,
    ui: AgentScreenUi,
    tools: ToolCardState,
    modifier: Modifier,
    onSendPlanEdit: (String) -> Unit,
) {
    when (item) {
        is LiveItem.Tools -> ToolRow(item.row, tools, modifier)
        is LiveItem.Report ->
            QualityReportCard(
                report = item.report,
                onOpenFinding = { _, _ -> },
                modifier = modifier,
            )
        LiveItem.Ask -> ui.cards.question?.let { question -> AskCard(ui, question, modifier) }
        LiveItem.Plan ->
            ui.cards.plan?.let { plan ->
                PlanCardSlot(ui, plan, modifier, onSendPlanEdit)
            }
    }
}

@Composable
private fun AskCard(
    ui: AgentScreenUi,
    question: QuestionUi,
    modifier: Modifier,
) {
    val cards = ui.loop.cards
    AskUserCard(
        question = question,
        answer = ui.cards.answer,
        draft = ui.cards.ui.draft,
        busy = ui.cards.ui.busy,
        failure = ui.cards.askFailure,
        onDraftChange = { cards.setDraft(it) },
        onOption = { cards.answerWithOption(it) },
        onSend = { cards.sendDraft() },
        modifier = modifier,
    )
}

@Composable
private fun PlanCardSlot(
    ui: AgentScreenUi,
    plan: PlanUi,
    modifier: Modifier,
    onSendPlanEdit: (String) -> Unit,
) {
    val cards = ui.loop.cards
    val state = ui.cards.ui
    PlanCard(
        plan = plan,
        started = ui.cards.planStarted,
        editing = state.editingPlan,
        planDraft = state.planDraft,
        runChoiceOpen = state.runChoiceOpen,
        collapsedSections = state.collapsedSections,
        busy = state.busy,
        failure = ui.cards.planFailure,
        onRun = { cards.openRunChoice() },
        onChooseMode = { mode -> cards.runPlan(mode) },
        onCancelRunChoice = { cards.closeRunChoice() },
        onStartEdit = { cards.startEditing() },
        onPlanDraftChange = { cards.setPlanDraft(it) },
        onCancelEdit = { cards.stopEditing() },
        onEdit = onSendPlanEdit,
        onKeepPlanning = { cards.keepPlanning() },
        onToggleSection = { cards.toggleSection(it) },
        modifier = modifier,
    )
}

/**
 * The approval bar for the first waiting approval, with its Always allow menu and deny sheet.
 * It draws nothing when no approval waits.
 */
@Composable
internal fun AgentApprovalSlot(
    ui: AgentScreenUi,
    modifier: Modifier = Modifier,
) {
    val state = ui.approvals
    val approval = state.approvals.firstOrNull() ?: return
    val vm = ui.loop.approvals
    val id = approval.toolCallId
    ApprovalBar(
        approval = approval,
        position = 1,
        total = state.approvals.size,
        expanded = state.expandedId == id,
        busy = state.busy,
        failure = state.failure,
        onToggleExpanded = { vm.toggleExpanded(id) },
        onAllow = { vm.allow(id) },
        onDeny = { vm.openDeny(id) },
        onAlwaysAllow = { vm.openAlwaysAllow(id) },
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    )
    ApprovalSheetSlot(state, vm)
}

/** The sheet the approval view model has open, if any. */
@Composable
private fun ApprovalSheetSlot(
    state: ApprovalScreenState,
    vm: ApprovalViewModel,
) {
    when (val sheet = state.sheet) {
        null -> Unit
        is ApprovalSheet.AlwaysAllow -> {
            val approval = state.approvals.firstOrNull { it.toolCallId == sheet.toolCallId }
            val rule = approval?.let { alwaysAllowRule(it.tool, it.target) }
            if (rule != null) {
                AlwaysAllowMenu(
                    rule = rule,
                    busy = state.busy,
                    failure = state.failure,
                    onConfirm = { vm.confirmAlwaysAllow(sheet.toolCallId) },
                    onCancel = { vm.closeSheet() },
                )
            }
        }
        is ApprovalSheet.Deny ->
            DenyReasonSheet(
                reason = state.reason,
                busy = state.busy,
                failure = state.failure,
                onReasonChange = { vm.editReason(it) },
                onSubmit = { vm.confirmDeny(sheet.toolCallId) },
                onCancel = { vm.closeSheet() },
            )
    }
}
