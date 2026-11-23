package app.tether.settings.mcp

import app.tether.core.generated.McpHeaderInput
import app.tether.core.generated.McpRuntime
import app.tether.core.generated.McpRuntimeException

/**
 * What the MCP screens need from the Rust core. Every call blocks, so callers run it off the main
 * thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface McpGateway {
    /** Every saved server, with its cached tools. Header values are never included. */
    fun servers(): List<McpServer>

    /** Saves a server and its headers, and returns the new server's id. */
    fun add(
        name: String,
        url: String,
        headers: List<McpHeader>,
    ): String

    /** Removes a server, its tools, and its saved headers. */
    fun remove(id: String)

    /** Turns one server on or off. */
    fun setEnabled(
        id: String,
        enabled: Boolean,
    )

    /** Saves one tool's enabled flag and approval. [approval] is sent as "ask" or "auto". */
    fun setTool(
        serverId: String,
        tool: String,
        enabled: Boolean,
        approval: McpApproval,
    )

    /** Reads the server's tool list again and caches it. */
    fun refreshTools(id: String)

    /** Connects to the server and returns how many tools it advertises. */
    fun test(id: String): Int
}

/**
 * The real gateway. It forwards each call to the UniFFI [McpRuntime] and turns each core error
 * into an [McpFailure]. The runtime opens on the first call, which runs off the main thread.
 */
class CoreMcpGateway(
    private val runtime: Lazy<McpRuntime>,
) : McpGateway {
    private val core: McpRuntime by runtime

    override fun servers(): List<McpServer> =
        guard { core.listMcpServers().map { it.toMcpServer() } }

    override fun add(
        name: String,
        url: String,
        headers: List<McpHeader>,
    ): String =
        guard {
            core.addMcpServer(
                name,
                url,
                headers.map { McpHeaderInput(it.name, it.value) },
            )
        }

    override fun remove(id: String) = guard { core.removeMcpServer(id) }

    override fun setEnabled(
        id: String,
        enabled: Boolean,
    ) = guard { core.setMcpServerEnabled(id, enabled) }

    override fun setTool(
        serverId: String,
        tool: String,
        enabled: Boolean,
        approval: McpApproval,
    ) = guard { core.setMcpTool(serverId, tool, enabled, approval.wire) }

    override fun refreshTools(id: String) = guard { core.refreshMcpTools(id) }

    override fun test(id: String): Int = guard { core.testMcpServer(id).toInt() }

    /**
     * Runs one core call. The core error is replaced on purpose: its text can carry a header value
     * or a reason, so only its [McpProblem] is kept.
     */
    @Suppress("SwallowedException")
    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: McpRuntimeException) {
            throw McpFailure(failure.problem())
        }
}
