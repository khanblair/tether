package app.tether.session.approval

/** What an approval asks the user to allow. Each kind has its own summary and icon. */
enum class ApprovalKind {
    Command,
    Server,
    ProcessStop,
    Edit,
    Delete,
    Move,
    Network,
    Push,
    ForcePush,
    PullRequest,
    GithubRepoCreate,
    GithubApi,
    Other,
}

/** The force push prefix the core puts on the target of a forced `git_push`. */
private const val FORCE_PUSH_PREFIX = "FORCE "

/**
 * The kind of an approval, from its tool name and target. The core names the tools; a forced
 * push has the prefix "FORCE " on its target. Any tool it does not know is [ApprovalKind.Other].
 */
fun approvalKind(
    tool: String,
    target: String,
): ApprovalKind =
    when {
        tool == "run" && target.endsWith(BACKGROUND_SUFFIX) -> ApprovalKind.Server
        tool == "git_push" && target.startsWith(FORCE_PUSH_PREFIX) -> ApprovalKind.ForcePush
        tool.startsWith("web_") -> ApprovalKind.Network
        else -> TOOL_KINDS[tool] ?: ApprovalKind.Other
    }

/** The kind of each tool name with one fixed kind. Names missing here are [ApprovalKind.Other]. */
private val TOOL_KINDS: Map<String, ApprovalKind> =
    mapOf(
        "run" to ApprovalKind.Command,
        "process_stop" to ApprovalKind.ProcessStop,
        "write_file" to ApprovalKind.Edit,
        "edit_file" to ApprovalKind.Edit,
        "multi_edit" to ApprovalKind.Edit,
        "delete_file" to ApprovalKind.Delete,
        "move_file" to ApprovalKind.Move,
        "git_push" to ApprovalKind.Push,
        "pr_create" to ApprovalKind.PullRequest,
        "github_repo_create" to ApprovalKind.GithubRepoCreate,
        "github_api" to ApprovalKind.GithubApi,
    )

/**
 * Shortens [text] to at most [maxChars] characters by cutting the middle and keeping both ends,
 * with an ellipsis mark in the cut. Paths keep their file name this way.
 */
fun middleEllipsize(
    text: String,
    maxChars: Int,
): String {
    require(maxChars >= 1) { "maxChars must be at least 1" }
    if (text.length <= maxChars) return text
    val keep = maxChars - 1
    val head = (keep + 1) / 2
    val tail = keep - head
    return text.take(head) + ELLIPSIS + text.takeLast(tail)
}

private const val ELLIPSIS = "…"
private const val BACKGROUND_SUFFIX = " (keeps running until stopped)"

/** The short target shown in the approval bar for a background server. */
internal fun approvalTarget(
    tool: String,
    target: String,
): String =
    if (tool == "run" && target.endsWith(BACKGROUND_SUFFIX)) {
        target
            .removePrefix("[linux] ")
            .removeSuffix(BACKGROUND_SUFFIX)
    } else {
        target
    }
