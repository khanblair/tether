package app.tether.session.tools

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.ToolCallUi

/**
 * A collapsed group of read-only calls, such as "Read 4 files, searched 2 times". Expanded, each
 * call shows as its own [ToolCallCard] below the summary. All expansion state is hoisted:
 * [isCallExpanded] and [onToggleCall] take a call id, and [groupId] keys the group itself.
 */
@Composable
fun ToolGroupCard(
    calls: List<ToolCallUi>,
    groupId: String,
    expanded: Boolean,
    onToggleGroup: (String) -> Unit,
    isCallExpanded: (String) -> Boolean,
    onToggleCall: (String) -> Unit,
    modifier: Modifier = Modifier,
    onRunElsewhere: (String) -> Unit = {},
) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        ToolCardSurface(modifier = Modifier) {
            ToolHeader(
                icon = TetherIcons.Search,
                expandable = true,
                expanded = expanded,
                onToggle = { onToggleGroup(groupId) },
                expandRes = R.string.tools_group_expand,
                collapseRes = R.string.tools_group_collapse,
            ) {
                GroupSummaryText(calls = calls)
            }
        }
        if (expanded) {
            calls.forEach { call ->
                ToolCallCard(
                    call = call,
                    expanded = isCallExpanded(call.id),
                    onToggle = { onToggleCall(call.id) },
                    onRunElsewhere = onRunElsewhere,
                )
            }
        }
    }
}

/** The summary sentence of a group, built from the non-zero parts. */
@Composable
private fun GroupSummaryText(calls: List<ToolCallUi>) {
    val parts = groupSummary(calls).parts()
    val texts =
        parts.map { (part, count) ->
            when (part) {
                GroupPart.Files -> pluralStringResource(R.plurals.tools_group_files, count, count)
                GroupPart.Folders ->
                    pluralStringResource(
                        R.plurals.tools_group_folders,
                        count,
                        count,
                    )
                GroupPart.Searches ->
                    pluralStringResource(
                        R.plurals.tools_group_searches,
                        count,
                        count,
                    )
                GroupPart.Checks -> pluralStringResource(R.plurals.tools_group_checks, count, count)
            }
        }
    Text(
        text = texts.joinToString(stringResource(R.string.tools_group_separator)),
        style = MaterialTheme.typography.labelLarge,
        color = TetherTheme.colors.text,
    )
}
