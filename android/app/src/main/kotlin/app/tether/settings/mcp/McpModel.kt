package app.tether.settings.mcp

import androidx.annotation.StringRes
import app.tether.R
import app.tether.core.generated.McpRuntimeException
import app.tether.core.generated.McpServerView
import app.tether.core.generated.McpToolView

/** The connection state of one server, as the core reports it. Unknown words count as not tried. */
enum class McpStatus {
    Connected,
    Failed,
    Disabled,
    NotTried,
    ;

    companion object {
        /** Maps the core's status word. */
        fun fromCore(value: String): McpStatus =
            when (value) {
                "connected" -> Connected
                "failed" -> Failed
                "disabled" -> Disabled
                else -> NotTried
            }
    }
}

/**
 * When a tool may run without asking. Any word other than "auto" counts as [Ask], the safe choice.
 */
enum class McpApproval(
    /** The word the core stores and expects. */
    val wire: String,
) {
    Ask("ask"),
    Auto("auto"),
    ;

    companion object {
        /** Maps the core's approval word. */
        fun fromCore(value: String): McpApproval = if (value == "auto") Auto else Ask
    }
}

/** One tool of a server. [description] is the server's text: show it as plain text only. */
data class McpTool(
    val name: String,
    val description: String,
    val enabled: Boolean,
    val approval: McpApproval,
)

/** One saved MCP server. Header values are never part of this type; only their names are. */
data class McpServer(
    val id: String,
    val name: String,
    val url: String,
    val enabled: Boolean,
    val status: McpStatus,
    val headerNames: List<String>,
    val tools: List<McpTool>,
)

/** The result of a connection test. [Connected] carries the tool count the server advertised. */
sealed interface McpTestResult {
    /** The server answered with [toolCount] tools. */
    data class Connected(
        val toolCount: Int,
    ) : McpTestResult

    /** The test could not complete. The screens show the same words for every cause. */
    data object Failed : McpTestResult
}

/** A problem an MCP call reports. The screens choose the words; see [messageRes]. */
enum class McpProblem {
    InvalidName,
    InvalidUrl,
    InvalidHeaderName,
    TooManyHeaders,
    HeaderTooLong,
    NotFound,
    InvalidApproval,
    Unavailable,
    TestFailed,
}

/**
 * An MCP call failed with [problem]. It keeps no cause on purpose: the core's text can carry a
 * header value or a reason, and no message should repeat it.
 */
class McpFailure(
    val problem: McpProblem,
) : Exception()

/**
 * A header the person typed. [toString] hides the value, so a logged state or failure never
 * carries it.
 */
class McpHeader(
    val name: String,
    val value: String,
) {
    override fun toString(): String = "McpHeader(name=$name, value=<hidden>)"
}

/** The core's view of a server, as the UI type. */
fun McpServerView.toMcpServer(): McpServer =
    McpServer(
        id = id,
        name = name,
        url = url,
        enabled = enabled,
        status = McpStatus.fromCore(status),
        headerNames = headerNames,
        tools = tools.map { it.toMcpTool() },
    )

/** The core's view of a tool, as the UI type. */
fun McpToolView.toMcpTool(): McpTool =
    McpTool(
        name = name,
        description = description,
        enabled = enabled,
        approval = McpApproval.fromCore(approval),
    )

/** The problem a core error stands for. No `else` branch, so a new core error breaks the build. */
internal fun McpRuntimeException.problem(): McpProblem =
    when (this) {
        is McpRuntimeException.InvalidName -> McpProblem.InvalidName
        is McpRuntimeException.InvalidUrl -> McpProblem.InvalidUrl
        is McpRuntimeException.InvalidHeaderName -> McpProblem.InvalidHeaderName
        is McpRuntimeException.TooManyHeaders -> McpProblem.TooManyHeaders
        is McpRuntimeException.HeaderTooLong -> McpProblem.HeaderTooLong
        is McpRuntimeException.NotFound -> McpProblem.NotFound
        is McpRuntimeException.InvalidApproval -> McpProblem.InvalidApproval
        is McpRuntimeException.Unavailable -> McpProblem.Unavailable
        is McpRuntimeException.TestFailed -> McpProblem.TestFailed
    }

/** The problem behind a failed call. Any other failure counts as [McpProblem.Unavailable]. */
internal fun Throwable.mcpProblem(): McpProblem =
    (this as? McpFailure)?.problem ?: McpProblem.Unavailable

/** The message for [problem]. Each one is a fixed string, so no core text reaches the screen. */
@StringRes
internal fun McpProblem.messageRes(): Int =
    when (this) {
        McpProblem.InvalidName -> R.string.mcp_error_invalid_name
        McpProblem.InvalidUrl -> R.string.mcp_error_invalid_url
        McpProblem.InvalidHeaderName -> R.string.mcp_error_invalid_header_name
        McpProblem.TooManyHeaders -> R.string.mcp_error_too_many_headers
        McpProblem.HeaderTooLong -> R.string.mcp_error_header_too_long
        McpProblem.NotFound -> R.string.mcp_error_not_found
        McpProblem.InvalidApproval -> R.string.mcp_error_invalid_approval
        McpProblem.Unavailable -> R.string.mcp_error_unavailable
        McpProblem.TestFailed -> R.string.mcp_error_test_failed
    }
