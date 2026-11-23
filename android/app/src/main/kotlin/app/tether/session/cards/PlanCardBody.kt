package app.tether.session.cards

import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.core.generated.PermissionModeView
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.PlanUi

/** The longest path the plan card shows before the middle is cut. */
private const val PATH_MAX_CHARS = 40

/** The numbered steps of the plan. */
@Composable
internal fun PlanSteps(steps: List<String>) {
    val colors = TetherTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        steps.forEachIndexed { index, step ->
            Row(verticalAlignment = Alignment.Top) {
                Text(
                    text = "${index + 1}.",
                    style = MaterialTheme.typography.bodyLarge,
                    color = colors.text2,
                    modifier = Modifier.width(TetherSize.icon),
                )
                Text(
                    text = step,
                    style = MaterialTheme.typography.bodyLarge,
                    color = colors.text,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

/**
 * The collapsible plan sections. Each one shows only when it has items. Files and commands use
 * the mono style, and long file paths are cut in the middle.
 */
@Composable
internal fun PlanSections(
    plan: PlanUi,
    collapsed: Set<PlanSection>,
    onToggle: (PlanSection) -> Unit,
) {
    val colors = TetherTheme.colors
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        if (plan.files.isNotEmpty()) {
            val count =
                pluralStringResource(
                    R.plurals.cards_plan_files_count,
                    plan.files.size,
                    plan.files.size,
                )
            PlanSectionBlock(
                title = stringResource(R.string.cards_plan_files),
                count = count,
                open = PlanSection.Files !in collapsed,
                onToggle = { onToggle(PlanSection.Files) },
            ) {
                plan.files.forEach { path ->
                    Text(
                        text = truncateMiddle(path, PATH_MAX_CHARS),
                        style = TetherTheme.mono,
                        color = colors.text,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
        if (plan.commands.isNotEmpty()) {
            val count =
                pluralStringResource(
                    R.plurals.cards_plan_commands_count,
                    plan.commands.size,
                    plan.commands.size,
                )
            PlanSectionBlock(
                title = stringResource(R.string.cards_plan_commands),
                count = count,
                open = PlanSection.Commands !in collapsed,
                onToggle = { onToggle(PlanSection.Commands) },
            ) {
                plan.commands.forEach { command ->
                    Text(text = command, style = TetherTheme.mono, color = colors.text)
                }
            }
        }
        if (plan.openQuestions.isNotEmpty()) {
            val count =
                pluralStringResource(
                    R.plurals.cards_plan_questions_count,
                    plan.openQuestions.size,
                    plan.openQuestions.size,
                )
            PlanSectionBlock(
                title = stringResource(R.string.cards_plan_questions),
                count = count,
                open = PlanSection.OpenQuestions !in collapsed,
                onToggle = { onToggle(PlanSection.OpenQuestions) },
            ) {
                plan.openQuestions.forEach { question ->
                    Text(
                        text = question,
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.text,
                    )
                }
            }
        }
    }
}

/** A section header that toggles, and its body when open. */
@Composable
private fun PlanSectionBlock(
    title: String,
    count: String,
    open: Boolean,
    onToggle: () -> Unit,
    content: @Composable () -> Unit,
) {
    val colors = TetherTheme.colors
    Column {
        Row(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .heightIn(min = TetherSize.touchMin)
                    .clickable(role = Role.Button, onClick = onToggle),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleSmall,
                color = colors.text,
                modifier = Modifier.weight(1f),
            )
            Text(text = count, style = MaterialTheme.typography.bodySmall, color = colors.text3)
            Spacer(Modifier.width(TetherSpace.space2))
            TetherIcon(
                icon = TetherIcons.ChevronDown,
                contentDescription = null,
                modifier = Modifier.rotate(if (open) HALF_TURN else 0f),
                tint = colors.text2,
            )
        }
        if (open) {
            Column(
                modifier =
                    Modifier.padding(
                        start = TetherSpace.space1,
                        bottom = TetherSpace.space2,
                    ),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
            ) {
                content()
            }
        }
    }
}

/** The mode choice of Run this plan: Ask, Accept edits, and Auto. Ask is the default. */
@Composable
internal fun RunModeChoice(
    enabled: Boolean,
    onChoose: (PermissionModeView) -> Unit,
    onCancel: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        Text(
            text = stringResource(R.string.cards_plan_choose_mode),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
        RUN_PLAN_MODES.forEach { mode ->
            RunModeRow(mode = mode, enabled = enabled, onClick = { onChoose(mode) })
        }
        TetherTextButton(
            label = stringResource(R.string.cards_action_cancel),
            onClick = onCancel,
        )
    }
}

/** One mode of the run choice, with its one-line description. */
@Composable
private fun RunModeRow(
    mode: PermissionModeView,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.sm)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .border(TetherBorder.input, colors.lineInput, shape)
                .clip(shape)
                .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
    ) {
        Text(
            text = stringResource(mode.labelRes()),
            style = MaterialTheme.typography.bodyLarge,
            color = if (enabled) colors.text else colors.textDisabled,
        )
        Text(
            text = stringResource(mode.descriptionRes()),
            style = MaterialTheme.typography.bodySmall,
            color = colors.text2,
        )
    }
}

/** The label of a run mode. */
internal fun PermissionModeView.labelRes(): Int =
    when (this) {
        PermissionModeView.ASK -> R.string.cards_plan_mode_ask
        PermissionModeView.ACCEPT_EDITS -> R.string.cards_plan_mode_accept_edits
        PermissionModeView.AUTO -> R.string.cards_plan_mode_auto
        PermissionModeView.PLAN, PermissionModeView.BYPASS -> error("A plan does not run in $this")
    }

/** The one-line description of a run mode. */
internal fun PermissionModeView.descriptionRes(): Int =
    when (this) {
        PermissionModeView.ASK -> R.string.cards_plan_mode_ask_desc
        PermissionModeView.ACCEPT_EDITS -> R.string.cards_plan_mode_accept_edits_desc
        PermissionModeView.AUTO -> R.string.cards_plan_mode_auto_desc
        PermissionModeView.PLAN, PermissionModeView.BYPASS -> error("A plan does not run in $this")
    }

private const val HALF_TURN = 180f
