package app.tether.settings.mcp

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BadgeTone
import app.tether.design.components.StatusBadge
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * One server in the list. The top part opens its detail. The bottom part has the Test button, the
 * test result, and the enable switch. The host shows only the address's host name here.
 */
@Composable
fun McpServerRow(
    server: McpServer,
    testing: Boolean,
    testResult: McpTestResult?,
    onOpen: () -> Unit,
    onToggle: (Boolean) -> Unit,
    onTest: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier =
            modifier.fillMaxWidth().padding(
                horizontal = TetherSpace.space4,
                vertical = TetherSpace.space3,
            ),
    ) {
        ServerSummary(server = server, onOpen = onOpen)
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TetherTextButton(
                label = stringResource(R.string.mcp_test),
                onClick = onTest,
                enabled = !testing,
                icon = TetherIcons.Zap,
            )
            Spacer(Modifier.width(TetherSpace.space2))
            TestOutcome(testing = testing, result = testResult, modifier = Modifier.weight(1f))
            val enableLabel = stringResource(R.string.mcp_enable_server, server.name)
            Switch(
                checked = server.enabled,
                onCheckedChange = onToggle,
                modifier =
                    Modifier.semantics { contentDescription = enableLabel },
            )
        }
    }
}

/** The name, host, status badge, and tool count. Tapping it opens the server. */
@Composable
private fun ServerSummary(
    server: McpServer,
    onOpen: () -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onOpen),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TetherIcon(icon = TetherIcons.Globe, contentDescription = null)
        Spacer(Modifier.width(TetherSpace.space3))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = server.name,
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text,
            )
            Text(
                text = hostOf(server.url),
                style = TetherTheme.monoSmall,
                color = TetherTheme.colors.text2,
            )
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
            ) {
                StatusBadge(
                    label = stringResource(server.status.labelRes()),
                    tone = server.status.tone(),
                    icon = server.status.icon(),
                )
                Text(
                    text = toolCountText(server.tools.size),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                )
            }
        }
        TetherIcon(
            icon = TetherIcons.ChevronRight,
            contentDescription = null,
            tint = TetherTheme.colors.text3,
        )
    }
}

/** The line after the Test button: the running label, or the result of the last test. */
@Composable
private fun TestOutcome(
    testing: Boolean,
    result: McpTestResult?,
    modifier: Modifier = Modifier,
) {
    val text: String
    val color: Color
    when {
        testing -> {
            text = stringResource(R.string.mcp_testing)
            color = TetherTheme.colors.text2
        }
        result is McpTestResult.Connected -> {
            text = testPassedText(result.toolCount)
            color = TetherTheme.colors.success
        }
        result == McpTestResult.Failed -> {
            text = stringResource(R.string.mcp_test_failed)
            color = TetherTheme.colors.danger
        }
        else -> return
    }
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = color,
        maxLines = 2,
        overflow = TextOverflow.Ellipsis,
        modifier = modifier.padding(start = TetherSpace.space1),
    )
}

/** "1 tool" or "3 tools", from the plural resource. */
@Composable
internal fun toolCountText(count: Int): String =
    pluralStringResource(R.plurals.mcp_tool_count, count, count)

/** "Connected. 3 tools found.", from the plural resource. */
@Composable
internal fun testPassedText(count: Int): String =
    pluralStringResource(R.plurals.mcp_test_passed, count, count)

/** The badge label of a status. */
internal fun McpStatus.labelRes(): Int =
    when (this) {
        McpStatus.Connected -> R.string.mcp_status_connected
        McpStatus.Failed -> R.string.mcp_status_failed
        McpStatus.Disabled -> R.string.mcp_status_disabled
        McpStatus.NotTried -> R.string.mcp_status_not_tried
    }

/** The badge tone of a status. Color is never the only signal: the badge has text and an icon. */
internal fun McpStatus.tone(): BadgeTone =
    when (this) {
        McpStatus.Connected -> BadgeTone.Success
        McpStatus.Failed -> BadgeTone.Danger
        McpStatus.Disabled, McpStatus.NotTried -> BadgeTone.Neutral
    }

/** The badge icon of a status. */
internal fun McpStatus.icon(): ImageVector =
    when (this) {
        McpStatus.Connected -> TetherIcons.CircleCheck
        McpStatus.Failed -> TetherIcons.CircleX
        McpStatus.Disabled -> TetherIcons.CirclePause
        McpStatus.NotTried -> TetherIcons.CircleDot
    }
