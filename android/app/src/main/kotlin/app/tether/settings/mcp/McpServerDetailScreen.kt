package app.tether.settings.mcp

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace
import app.tether.settings.SettingsPage

/**
 * One server and its tools. [onRemoved] runs once the server is gone, so the host can leave this
 * screen. Every other action goes to the view model.
 */
@Composable
fun McpServerDetailScreen(
    viewModel: McpServerDetailViewModel,
    onBack: () -> Unit,
    onRemoved: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.removed) {
        if (state.removed) onRemoved()
    }
    McpServerDetailContent(
        state = state,
        onBack = onBack,
        onToggleTool = viewModel::setToolEnabled,
        onApproval = viewModel::setToolApproval,
        onRefresh = viewModel::refreshTools,
        onAskRemove = viewModel::askRemove,
        onConfirmRemove = viewModel::confirmRemove,
        onCancelRemove = viewModel::cancelRemove,
        onDismissProblem = viewModel::dismissProblem,
    )
}

/** The detail screen without a view model, so previews can show each state. */
@Composable
internal fun McpServerDetailContent(
    state: McpServerDetailUiState,
    onBack: () -> Unit,
    onToggleTool: (String, Boolean) -> Unit,
    onApproval: (String, McpApproval) -> Unit,
    onRefresh: () -> Unit,
    onAskRemove: () -> Unit,
    onConfirmRemove: () -> Unit,
    onCancelRemove: () -> Unit,
    onDismissProblem: () -> Unit,
) {
    val server = state.server
    SettingsPage(title = server?.name ?: stringResource(R.string.mcp_title), onBack = onBack) {
        when {
            state.loading -> Unit
            server == null -> state.loadProblem?.let { LoadProblemBanner(it) }
            else ->
                ServerBody(
                    server = server,
                    state = state,
                    onToggleTool = onToggleTool,
                    onApproval = onApproval,
                    onRefresh = onRefresh,
                    onAskRemove = onAskRemove,
                    onDismissProblem = onDismissProblem,
                )
        }
    }
    if (server != null && state.confirmingRemove) {
        RemoveServerSheet(
            name = server.name,
            busy = state.busy,
            onConfirm = onConfirmRemove,
            onCancel = onCancelRemove,
        )
    }
}

/** The address, the tools, the approval note, and the actions. */
@Composable
private fun ServerBody(
    server: McpServer,
    state: McpServerDetailUiState,
    onToggleTool: (String, Boolean) -> Unit,
    onApproval: (String, McpApproval) -> Unit,
    onRefresh: () -> Unit,
    onAskRemove: () -> Unit,
    onDismissProblem: () -> Unit,
) {
    SectionTitle(text = stringResource(R.string.mcp_detail_address))
    Text(
        text = server.url,
        style = TetherTheme.mono,
        color = TetherTheme.colors.text,
        modifier = Modifier.fillMaxWidth(),
    )
    state.problem?.let { problem ->
        InlineBanner(
            text = stringResource(problem.messageRes()),
            tone = BannerTone.Danger,
            actionLabel = stringResource(R.string.mcp_dismiss),
            onAction = onDismissProblem,
        )
    }
    SectionTitle(text = stringResource(R.string.mcp_detail_tools_title))
    if (server.tools.isEmpty()) {
        Text(
            text = stringResource(R.string.mcp_detail_no_tools),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
    } else {
        ListGroup {
            server.tools.forEachIndexed { index, tool ->
                if (index > 0) {
                    HorizontalDivider(
                        thickness = TetherBorder.hairline,
                        color = TetherTheme.colors.line,
                    )
                }
                McpToolRow(
                    tool = tool,
                    enabled = !state.busy,
                    onToggle = { onToggleTool(tool.name, it) },
                    onApproval = { onApproval(tool.name, it) },
                )
            }
        }
    }
    Text(
        text = stringResource(R.string.mcp_approval_note),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
    )
    TetherTextButton(
        label = stringResource(R.string.mcp_refresh_tools),
        onClick = onRefresh,
        enabled = !state.busy,
    )
    TetherTextButton(
        label = stringResource(R.string.mcp_remove_server),
        onClick = onAskRemove,
        enabled = !state.busy,
        tone = TextButtonTone.Danger,
    )
}

/** The banner for a server that is missing or could not be read. */
@Composable
private fun LoadProblemBanner(problem: McpProblem) {
    InlineBanner(text = stringResource(problem.messageRes()), tone = BannerTone.Danger)
}

/** The remove confirmation. It names the server, says what goes, and offers a way back. */
@Composable
private fun RemoveServerSheet(
    name: String,
    busy: Boolean,
    onConfirm: () -> Unit,
    onCancel: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.mcp_remove_title, name), onDismiss = onCancel) {
        Text(
            text = stringResource(R.string.mcp_remove_body),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
            modifier = Modifier.padding(horizontal = TetherSpace.space4),
        )
        Column(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
            TetherTextButton(
                label = stringResource(R.string.mcp_remove_confirm),
                onClick = onConfirm,
                enabled = !busy,
                tone = TextButtonTone.Danger,
                modifier = Modifier.fillMaxWidth(),
            )
            TetherTextButton(
                label = stringResource(R.string.mcp_remove_keep),
                onClick = onCancel,
                modifier = Modifier.fillMaxWidth(),
            )
        }
    }
}
