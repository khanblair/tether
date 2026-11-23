package app.tether.settings.mcp

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * One tool: its name, its description as plain text, the enable switch, and the approval choice.
 * [description] is the server's text, so it is shown as is and cut to [MCP_DESCRIPTION_MAX].
 */
@Composable
fun McpToolRow(
    tool: McpTool,
    enabled: Boolean,
    onToggle: (Boolean) -> Unit,
    onApproval: (McpApproval) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier =
            modifier.fillMaxWidth().padding(
                horizontal = TetherSpace.space4,
                vertical = TetherSpace.space3,
            ),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        val enableLabel = stringResource(R.string.mcp_tool_enabled, tool.name)
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = tool.name,
                    style = TetherTheme.monoLabel,
                    color = TetherTheme.colors.text,
                )
                Text(
                    text = toolDescriptionText(tool.description),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text2,
                    maxLines = DESCRIPTION_LINES,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Switch(
                checked = tool.enabled,
                onCheckedChange = onToggle,
                enabled = enabled,
                modifier = Modifier.semantics { contentDescription = enableLabel },
            )
        }
        ApprovalChoice(selected = tool.approval, enabled = enabled, onChoose = onApproval)
    }
}

/** The two approval options side by side. The chosen one shows a check and says it is selected. */
@Composable
private fun ApprovalChoice(
    selected: McpApproval,
    enabled: Boolean,
    onChoose: (McpApproval) -> Unit,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        ApprovalOption(
            label = stringResource(R.string.mcp_approval_ask),
            selected = selected == McpApproval.Ask,
            enabled = enabled,
            onClick = { onChoose(McpApproval.Ask) },
        )
        ApprovalOption(
            label = stringResource(R.string.mcp_approval_auto),
            selected = selected == McpApproval.Auto,
            enabled = enabled,
            onClick = { onChoose(McpApproval.Auto) },
        )
    }
}

/** One approval option: a 48 dp target, a tonal fill when chosen, a hairline border when not. */
@Composable
private fun ApprovalOption(
    label: String,
    selected: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.sm)
    val ink = if (selected) TetherTheme.colors.onTonal else TetherTheme.colors.text2
    Row(
        modifier =
            Modifier
                .heightIn(min = TetherSize.touchMin)
                .clip(shape)
                .background(if (selected) TetherTheme.colors.tonal else Color.Transparent)
                .then(
                    if (selected) {
                        Modifier
                    } else {
                        Modifier.border(TetherBorder.input, TetherTheme.colors.lineInput, shape)
                    },
                ).selectable(
                    selected = selected,
                    enabled = enabled,
                    role = Role.RadioButton,
                    onClick = onClick,
                ).padding(horizontal = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (selected) {
            TetherIcon(
                icon = TetherIcons.CircleCheck,
                contentDescription = null,
                tint = ink,
                size = TetherSize.iconXs,
            )
            Spacer(Modifier.width(TetherSpace.space1))
        }
        Text(text = label, style = MaterialTheme.typography.labelLarge, color = ink)
    }
}

/** The description to show: the server's text as plain text, or a note when it is empty. */
@Composable
private fun toolDescriptionText(description: String): String =
    if (description.isBlank()) {
        stringResource(R.string.mcp_tool_no_description)
    } else {
        plainDescription(description)
    }

private const val DESCRIPTION_LINES = 4
