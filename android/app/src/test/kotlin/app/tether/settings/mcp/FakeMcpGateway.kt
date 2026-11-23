package app.tether.settings.mcp

/** One call to [FakeMcpGateway.setTool], with the approval as the core receives it. */
data class ToolCall(
    val serverId: String,
    val tool: String,
    val enabled: Boolean,
    val approval: String,
)

/** One call to [FakeMcpGateway.add]. The header values are kept to prove they reach the gateway. */
data class AddRequest(
    val name: String,
    val url: String,
    val headers: List<Pair<String, String>>,
)

/** A gateway for unit tests. It keeps servers in memory and records every call. */
class FakeMcpGateway : McpGateway {
    /** The servers the fake returns. Tests change this to simulate the core saving a change. */
    val serverList = mutableListOf<McpServer>()

    /** Every call by name, in order. */
    val calls = mutableListOf<String>()

    /** Every tool change, in order. */
    val toolCalls = mutableListOf<ToolCall>()

    /** Every add request, in order. */
    val addRequests = mutableListOf<AddRequest>()

    /** Throws this from the next call of any kind, then clears itself. */
    var failNext: Exception? = null

    /** Throws this from the next [test] call only, so a save can succeed and its test fail. */
    var failTest: Exception? = null

    /** The tool count [test] returns. */
    var testToolCount: Int = DEFAULT_TOOL_COUNT

    /** The id [add] returns. */
    var nextId: String = DEFAULT_ID

    private fun record(call: String) {
        calls += call
        failNext?.let {
            failNext = null
            throw it
        }
    }

    override fun servers(): List<McpServer> {
        record("servers")
        return serverList.toList()
    }

    override fun add(
        name: String,
        url: String,
        headers: List<McpHeader>,
    ): String {
        record("add")
        addRequests += AddRequest(name, url, headers.map { it.name to it.value })
        return nextId
    }

    override fun remove(id: String) {
        record("remove:$id")
        serverList.removeAll { it.id == id }
    }

    override fun setEnabled(
        id: String,
        enabled: Boolean,
    ) {
        record("setEnabled:$id:$enabled")
        serverList.replaceAll { if (it.id == id) it.copy(enabled = enabled) else it }
    }

    override fun setTool(
        serverId: String,
        tool: String,
        enabled: Boolean,
        approval: McpApproval,
    ) {
        record("setTool:$serverId:$tool")
        toolCalls += ToolCall(serverId, tool, enabled, approval.wire)
        serverList.replaceAll { server ->
            if (server.id != serverId) {
                server
            } else {
                server.copy(
                    tools =
                        server.tools.map {
                            if (it.name ==
                                tool
                            ) {
                                it.copy(enabled = enabled, approval = approval)
                            } else {
                                it
                            }
                        },
                )
            }
        }
    }

    override fun refreshTools(id: String) {
        record("refreshTools:$id")
    }

    override fun test(id: String): Int {
        record("test:$id")
        failTest?.let {
            failTest = null
            throw it
        }
        return testToolCount
    }

    companion object {
        /** The tool count a test reports unless a test changes it. */
        const val DEFAULT_TOOL_COUNT = 3

        /** The id a new server gets unless a test changes it. */
        const val DEFAULT_ID = "docs-search"
    }
}

/** A saved server for tests, with one tool that asks before it runs. */
fun sampleServer(
    id: String = FakeMcpGateway.DEFAULT_ID,
    enabled: Boolean = true,
    tools: List<McpTool> =
        listOf(McpTool("search_docs", "Search the docs.", true, McpApproval.Ask)),
): McpServer =
    McpServer(
        id = id,
        name = id,
        url = "https://mcp.example.com/docs",
        enabled = enabled,
        status = if (enabled) McpStatus.Connected else McpStatus.Disabled,
        headerNames = listOf("Authorization"),
        tools = tools,
    )
