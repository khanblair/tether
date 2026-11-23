package app.tether.session.cards

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.SessionState
import app.tether.design.components.StatusMark
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherIconButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.TodoStatus
import app.tether.session.agent.TodoUi

/**
 * The checklist of the agent, pinned above the transcript. Collapsed it shows the progress and the
 * current item. Expanded it lists every item with its status. When every item is done it says
 * "All done" and can be dismissed. Items are plain text.
 */
@Composable
fun ChecklistPin(
    items: List<TodoUi>,
    current: TodoUi?,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    if (items.isEmpty()) return
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .background(colors.surface, shape)
                .border(TetherBorder.hairline, colors.line, shape)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
    ) {
        ChecklistHeader(
            items = items,
            current = current,
            expanded = expanded,
            onToggleExpanded = onToggleExpanded,
            onDismiss = onDismiss,
        )
        if (expanded) {
            items.forEach { ChecklistItemRow(it) }
        }
    }
}

/** The header row: the progress, the current item, a chevron, and Dismiss once all are done. */
@Composable
private fun ChecklistHeader(
    items: List<TodoUi>,
    current: TodoUi?,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = TetherTheme.colors
    val allDone = checklistAllDone(items)
    val toggleLabel =
        stringResource(
            if (expanded) R.string.cards_checklist_collapse else R.string.cards_checklist_expand,
        )
    Row(verticalAlignment = Alignment.CenterVertically) {
        Column(
            modifier =
                Modifier
                    .weight(1f)
                    .heightIn(min = TetherSize.touchMin)
                    .clickable(
                        role = Role.Button,
                        onClickLabel = toggleLabel,
                        onClick = onToggleExpanded,
                    ).padding(vertical = TetherSpace.space2),
        ) {
            ChecklistSummary(items, current, expanded)
        }
        TetherIcon(
            icon = TetherIcons.ChevronDown,
            contentDescription = null,
            modifier = Modifier.rotate(if (expanded) HALF_TURN else 0f),
            tint = colors.text2,
        )
        if (allDone) {
            Spacer(Modifier.width(TetherSpace.space2))
            TetherIconButton(
                icon = TetherIcons.X,
                contentDescription = stringResource(R.string.cards_checklist_dismiss),
                onClick = onDismiss,
            )
        }
    }
}

/** The progress line and, while collapsed, the current item. */
@Composable
private fun ChecklistSummary(
    items: List<TodoUi>,
    current: TodoUi?,
    expanded: Boolean,
) {
    val colors = TetherTheme.colors
    if (checklistAllDone(items)) {
        Text(
            text = stringResource(R.string.cards_checklist_all_done),
            style = MaterialTheme.typography.bodyMedium,
            color = colors.text,
        )
        return
    }
    val counts = checklistCounts(items)
    Text(
        text = stringResource(R.string.cards_checklist_progress, counts.done, counts.total),
        style = MaterialTheme.typography.bodySmall,
        color = colors.text2,
    )
    if (!expanded && current != null) {
        Text(
            text = current.text,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.text,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

/** One item: its status mark, then its text. Completed text is secondary, never struck through. */
@Composable
private fun ChecklistItemRow(item: TodoUi) {
    val colors = TetherTheme.colors
    val statusLabel =
        stringResource(
            when (item.status) {
                TodoStatus.Pending -> R.string.cards_checklist_status_pending
                TodoStatus.InProgress -> R.string.cards_checklist_status_running
                TodoStatus.Completed -> R.string.cards_checklist_status_done
            },
        )
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(vertical = TetherSpace.space2)
                .semantics(mergeDescendants = true) { stateDescription = statusLabel },
        verticalAlignment = Alignment.Top,
    ) {
        StatusMark(state = item.status.toSessionState())
        Spacer(Modifier.width(TetherSpace.space3))
        Text(
            text = item.text,
            style = MaterialTheme.typography.bodyMedium,
            color = if (item.status == TodoStatus.Completed) colors.text2 else colors.text,
            modifier = Modifier.weight(1f),
        )
    }
}

/** The status mark that shows a checklist status. */
internal fun TodoStatus.toSessionState(): SessionState =
    when (this) {
        TodoStatus.Pending -> SessionState.Idle
        TodoStatus.InProgress -> SessionState.Running
        TodoStatus.Completed -> SessionState.Done
    }

private const val HALF_TURN = 180f
