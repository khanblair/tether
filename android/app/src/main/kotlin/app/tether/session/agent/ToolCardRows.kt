package app.tether.session.agent

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import app.tether.session.tools.CallRow
import app.tether.session.tools.RUN_TOOL
import app.tether.session.tools.RunCard
import app.tether.session.tools.ToolCallCard
import app.tether.session.tools.ToolGroupCard
import app.tether.session.tools.groupCalls

/** The id of the group card for [row]: "group:" plus the id of its first call. */
internal fun groupCardId(row: CallRow.Group): String = "group:${row.calls.first().id}"

/** The tool cards of a run of calls, grouped the same way as the live section. */
@Composable
internal fun ToolCallRows(
    calls: List<ToolCallUi>,
    tools: ToolCardState,
) {
    groupCalls(calls).forEach { row -> ToolRow(row, tools) }
}

/** One row of tool cards: a single call, or a group of read-only calls. */
@Composable
internal fun ToolRow(
    row: CallRow,
    tools: ToolCardState,
    modifier: Modifier = Modifier,
) {
    when (row) {
        is CallRow.Single -> ToolCardFor(row.call, tools, modifier)
        is CallRow.Group ->
            ToolGroupCard(
                calls = row.calls,
                groupId = groupCardId(row),
                expanded = groupCardId(row) in tools.expanded,
                onToggleGroup = tools.onToggle,
                isCallExpanded = { it in tools.expanded },
                onToggleCall = tools.onToggle,
                modifier = modifier,
            )
    }
}

/** The card of one call: a run card for `run`, and a tool call card for the rest. */
@Composable
private fun ToolCardFor(
    call: ToolCallUi,
    tools: ToolCardState,
    modifier: Modifier,
) {
    val expanded = call.id in tools.expanded
    val onToggle: () -> Unit = { tools.onToggle(call.id) }
    val running = call.status == ToolStatus.Running
    if (call.tool == RUN_TOOL) {
        RunCard(
            call = call,
            expanded = expanded,
            onToggle = onToggle,
            modifier = modifier,
            animateRunning = running,
        )
    } else {
        ToolCallCard(
            call = call,
            expanded = expanded,
            onToggle = onToggle,
            modifier = modifier,
            animateRunning = running,
        )
    }
}
