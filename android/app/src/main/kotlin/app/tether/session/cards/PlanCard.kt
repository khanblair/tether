package app.tether.session.cards

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.core.generated.PermissionModeView
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.PlanUi

/**
 * The plan from Plan mode: the summary, the numbered steps, and collapsible sections. Its
 * buttons are Run this plan (the one accent, which opens a mode choice), Edit plan, and Keep
 * planning. After a run the card shows "Plan started" and no buttons.
 */
@Composable
fun PlanCard(
    plan: PlanUi,
    started: Boolean,
    editing: Boolean,
    planDraft: String,
    runChoiceOpen: Boolean,
    collapsedSections: Set<PlanSection>,
    busy: Boolean,
    failure: AgentLoopFailure?,
    onRun: () -> Unit,
    onChooseMode: (PermissionModeView) -> Unit,
    onCancelRunChoice: () -> Unit,
    onStartEdit: () -> Unit,
    onPlanDraftChange: (String) -> Unit,
    onCancelEdit: () -> Unit,
    onEdit: (String) -> Unit,
    onKeepPlanning: () -> Unit,
    onToggleSection: (PlanSection) -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(colors.surface, shape)
                .border(TetherBorder.hairline, colors.line, shape)
                .padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        PlanHeader()
        Text(text = plan.summary, style = MaterialTheme.typography.bodyLarge, color = colors.text)
        if (editing) {
            PlanEditor(planDraft, busy, onPlanDraftChange, onCancelEdit, onEdit)
        } else {
            PlanSteps(plan.steps)
            PlanSections(plan, collapsedSections, onToggleSection)
        }
        if (started) {
            Text(
                text = stringResource(R.string.cards_plan_started),
                style = MaterialTheme.typography.titleSmall,
                color = colors.text2,
            )
        } else if (!editing) {
            PlanActions(
                runChoiceOpen,
                busy,
                onRun,
                onChooseMode,
                onCancelRunChoice,
                onStartEdit,
                onKeepPlanning,
            )
        }
        if (failure != null) {
            InlineBanner(text = stringResource(failure.messageRes()), tone = BannerTone.Danger)
        }
    }
}

/** The card title, with the plan icon. The icon is decorative; the title names it. */
@Composable
private fun PlanHeader() {
    Row(verticalAlignment = Alignment.CenterVertically) {
        TetherIcon(
            icon = TetherIcons.FileText,
            contentDescription = null,
            tint = TetherTheme.colors.text2,
        )
        Text(
            text = stringResource(R.string.cards_plan_title),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
            modifier = Modifier.padding(start = TetherSpace.space2),
        )
    }
}

/** The buttons under the plan, or the mode choice when Run this plan was tapped. */
@Composable
private fun PlanActions(
    runChoiceOpen: Boolean,
    busy: Boolean,
    onRun: () -> Unit,
    onChooseMode: (PermissionModeView) -> Unit,
    onCancelRunChoice: () -> Unit,
    onStartEdit: () -> Unit,
    onKeepPlanning: () -> Unit,
) {
    if (runChoiceOpen) {
        RunModeChoice(enabled = !busy, onChoose = onChooseMode, onCancel = onCancelRunChoice)
        return
    }
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        PrimaryButton(
            label = stringResource(R.string.cards_plan_run),
            onClick = onRun,
            modifier = Modifier.fillMaxWidth(),
            enabled = !busy,
            loading = busy,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
            TetherTextButton(
                label = stringResource(R.string.cards_plan_edit),
                onClick = onStartEdit,
                enabled = !busy,
            )
            TetherTextButton(
                label = stringResource(R.string.cards_plan_keep),
                onClick = onKeepPlanning,
                enabled = !busy,
            )
        }
    }
}

/** The plan text as an editable field. Save sends the edited plan back as a user message. */
@Composable
private fun PlanEditor(
    planDraft: String,
    busy: Boolean,
    onDraftChange: (String) -> Unit,
    onCancel: () -> Unit,
    onSave: (String) -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        OutlinedTextField(
            value = planDraft,
            onValueChange = onDraftChange,
            modifier = Modifier.fillMaxWidth().heightIn(min = TetherSize.touchMin),
            enabled = !busy,
            label = { Text(text = stringResource(R.string.cards_plan_edit_label)) },
            textStyle = MaterialTheme.typography.bodyLarge,
            minLines = 4,
            maxLines = 12,
            shape = RoundedCornerShape(TetherRadius.sm),
        )
        Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
            PrimaryButton(
                label = stringResource(R.string.cards_plan_save),
                onClick = { onSave(planDraft.trim()) },
                enabled = !busy && planDraft.isNotBlank(),
                loading = busy,
            )
            TetherTextButton(
                label = stringResource(R.string.cards_action_cancel),
                onClick = onCancel,
                enabled = !busy,
            )
        }
    }
}
