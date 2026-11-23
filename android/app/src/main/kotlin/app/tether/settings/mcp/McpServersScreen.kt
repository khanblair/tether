package app.tether.settings.mcp

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherBorder
import app.tether.settings.SettingsPage

/**
 * The MCP server list. It reads [viewModel] and passes each tap up: [onOpenServer] with the
 * server's id, and [onAddServer] to open the add sheet, which the host shows.
 */
@Composable
fun McpServersScreen(
    viewModel: McpServersViewModel,
    onBack: () -> Unit,
    onOpenServer: (String) -> Unit,
    onAddServer: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    McpServersContent(
        state = state,
        onBack = onBack,
        onOpenServer = onOpenServer,
        onAddServer = onAddServer,
        onToggle = viewModel::setEnabled,
        onTest = viewModel::test,
        onRetry = viewModel::reload,
        onDismissProblem = viewModel::dismissProblem,
    )
}

/** The list screen without a view model, so previews can show each state. */
@Composable
internal fun McpServersContent(
    state: McpServersUiState,
    onBack: () -> Unit,
    onOpenServer: (String) -> Unit,
    onAddServer: () -> Unit,
    onToggle: (String, Boolean) -> Unit,
    onTest: (String) -> Unit,
    onRetry: () -> Unit,
    onDismissProblem: () -> Unit,
) {
    SettingsPage(title = stringResource(R.string.mcp_title), onBack = onBack) {
        Text(
            text = stringResource(R.string.mcp_body),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
        if (state.actionProblem != null) {
            InlineBanner(
                text = stringResource(R.string.mcp_action_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.mcp_dismiss),
                onAction = onDismissProblem,
            )
        }
        when {
            state.loading -> Unit
            state.loadFailed -> LoadFailed(onRetry)
            state.servers.isEmpty() -> EmptyList(onAddServer)
            else -> ServerList(state, onOpenServer, onToggle, onTest, onAddServer)
        }
    }
}

/** The list of servers and the add action under it. */
@Composable
private fun ServerList(
    state: McpServersUiState,
    onOpenServer: (String) -> Unit,
    onToggle: (String, Boolean) -> Unit,
    onTest: (String) -> Unit,
    onAddServer: () -> Unit,
) {
    ListGroup {
        state.servers.forEachIndexed { index, server ->
            if (index > 0) {
                HorizontalDivider(
                    thickness = TetherBorder.hairline,
                    color = TetherTheme.colors.line,
                )
            }
            McpServerRow(
                server = server,
                testing = server.id in state.testing,
                testResult = state.testResults[server.id],
                onOpen = { onOpenServer(server.id) },
                onToggle = { onToggle(server.id, it) },
                onTest = { onTest(server.id) },
            )
        }
    }
    TetherTextButton(
        label = stringResource(R.string.mcp_add_server),
        onClick = onAddServer,
        icon = TetherIcons.Plus,
        modifier = Modifier.fillMaxWidth(),
    )
}

/** The empty state: one sentence and the one action that fills it. */
@Composable
private fun EmptyList(onAddServer: () -> Unit) {
    Text(
        text = stringResource(R.string.mcp_empty_body),
        style = MaterialTheme.typography.bodyMedium,
        color = TetherTheme.colors.text2,
    )
    PrimaryButton(
        label = stringResource(R.string.mcp_add_server),
        onClick = onAddServer,
        modifier = Modifier.fillMaxWidth(),
    )
}

/** The list could not load. The retry action reads it again. */
@Composable
private fun LoadFailed(onRetry: () -> Unit) {
    InlineBanner(text = stringResource(R.string.mcp_load_failed), tone = BannerTone.Danger)
    TetherTextButton(label = stringResource(R.string.mcp_try_again), onClick = onRetry)
}
