package app.tether.settings.mcp

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace

private val sampleTools =
    listOf(
        McpTool("search_docs", "Search the project docs by keyword.", true, McpApproval.Ask),
        McpTool("read_page", "Read one docs page as text.", true, McpApproval.Auto),
        McpTool("list_versions", "List the published versions.", false, McpApproval.Ask),
    )

private val sampleServers =
    listOf(
        McpServer(
            id = "docs-search",
            name = "docs-search",
            url = "https://mcp.example.com/docs",
            enabled = true,
            status = McpStatus.Connected,
            headerNames = listOf("Authorization"),
            tools = sampleTools,
        ),
        McpServer(
            id = "issues",
            name = "issues",
            url = "https://issues.example.com/mcp",
            enabled = false,
            status = McpStatus.Disabled,
            headerNames = emptyList(),
            tools = emptyList(),
        ),
        McpServer(
            id = "wiki",
            name = "wiki",
            url = "https://wiki.example.com/mcp",
            enabled = true,
            status = McpStatus.Failed,
            headerNames = listOf("X-Team"),
            tools = emptyList(),
        ),
    )

private const val LONG_TEXT_REPEATS = 20
private const val SAMPLE_TOOL_COUNT = 3

private val longDescriptionTool =
    McpTool(
        name = "run_query",
        description = "Runs a read-only query. ".repeat(LONG_TEXT_REPEATS),
        enabled = true,
        approval = McpApproval.Auto,
    )

private fun listState(
    servers: List<McpServer> = sampleServers,
    loading: Boolean = false,
    loadFailed: Boolean = false,
    testResults: Map<String, McpTestResult> = emptyMap(),
) = McpServersUiState(
    servers = servers,
    loading = loading,
    loadFailed = loadFailed,
    testResults = testResults,
)

@Preview(name = "MCP list, populated, light", showBackground = true)
@Composable
internal fun McpListLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        McpListSample(
            listState(
                testResults =
                    mapOf(
                        "docs-search" to McpTestResult.Connected(SAMPLE_TOOL_COUNT),
                        "wiki" to McpTestResult.Failed,
                    ),
            ),
        )
    }
}

@Preview(name = "MCP list, populated, dark", showBackground = true)
@Composable
internal fun McpListDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { McpListSample(listState()) }
}

@Preview(name = "MCP list, empty, light", showBackground = true)
@Composable
internal fun McpListEmptyPreview() {
    TetherTheme(mode = ThemeMode.Light) { McpListSample(listState(servers = emptyList())) }
}

@Preview(name = "MCP list, could not load, dark", showBackground = true)
@Composable
internal fun McpListFailedPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        McpListSample(listState(servers = emptyList(), loading = false, loadFailed = true))
    }
}

@Composable
private fun McpListSample(state: McpServersUiState) {
    Column(modifier = Modifier.padding(TetherSpace.space4)) {
        McpServersContent(
            state = state,
            onBack = {},
            onOpenServer = {},
            onAddServer = {},
            onToggle = { _, _ -> },
            onTest = {},
            onRetry = {},
            onDismissProblem = {},
        )
    }
}

@Preview(name = "MCP server detail, tools, light", showBackground = true)
@Composable
internal fun McpDetailLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        McpDetailSample(
            McpServerDetailUiState(loading = false, server = sampleServers.first()),
        )
    }
}

@Preview(name = "MCP server detail, long description, dark", showBackground = true)
@Composable
internal fun McpDetailDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        McpDetailSample(
            McpServerDetailUiState(
                loading = false,
                server = sampleServers.first().copy(tools = listOf(longDescriptionTool)),
            ),
        )
    }
}

@Preview(name = "MCP server detail, remove sheet, light", showBackground = true)
@Composable
internal fun McpDetailRemovePreview() {
    TetherTheme(mode = ThemeMode.Light) {
        McpDetailSample(
            McpServerDetailUiState(
                loading = false,
                server = sampleServers.first(),
                confirmingRemove = true,
            ),
        )
    }
}

@Composable
private fun McpDetailSample(state: McpServerDetailUiState) {
    McpServerDetailContent(
        state = state,
        onBack = {},
        onToggleTool = { _, _ -> },
        onApproval = { _, _ -> },
        onRefresh = {},
        onAskRemove = {},
        onConfirmRemove = {},
        onCancelRemove = {},
        onDismissProblem = {},
    )
}

@Preview(name = "Add MCP server, empty, light", showBackground = true)
@Composable
internal fun AddMcpEmptyPreview() {
    TetherTheme(mode = ThemeMode.Light) { AddMcpSample(AddMcpServerUiState()) }
}

@Preview(name = "Add MCP server, invalid input, dark", showBackground = true)
@Composable
internal fun AddMcpInvalidPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AddMcpSample(
            AddMcpServerUiState(
                name = "Docs Search",
                url = "http://mcp.example.com",
                headers = listOf(HeaderDraft(name = "X Team", value = "team-value")),
            ),
        )
    }
}

@Preview(name = "Add MCP server, saved and connected, light", showBackground = true)
@Composable
internal fun AddMcpSavedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AddMcpSample(
            AddMcpServerUiState(
                name = "docs-search",
                url = "https://mcp.example.com/docs",
                addedId = "docs-search",
                testResult = McpTestResult.Connected(SAMPLE_TOOL_COUNT),
            ),
        )
    }
}

@Preview(name = "Add MCP server, saved, test failed, dark", showBackground = true)
@Composable
internal fun AddMcpSavedFailedPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AddMcpSample(
            AddMcpServerUiState(
                name = "wiki",
                url = "https://wiki.example.com/mcp",
                addedId = "wiki",
                testResult = McpTestResult.Failed,
            ),
        )
    }
}

@Composable
private fun AddMcpSample(state: AddMcpServerUiState) {
    val noop = {}
    AddMcpServerForm(
        state = state,
        callbacks =
            AddMcpServerCallbacks(
                onNameChange = {},
                onUrlChange = {},
                onHeaderNameChange = { _, _ -> },
                onHeaderValueChange = { _, _ -> },
                onToggleReveal = {},
                onAddHeader = noop,
                onRemoveHeader = {},
                onAdd = noop,
                onDismissProblem = noop,
            ),
        onDone = noop,
    )
}
