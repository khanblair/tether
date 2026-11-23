package app.tether.session.approval

/** What an approval asks the user to allow. Each kind has its own summary and icon. */
enum class ApprovalKind {
    Command,
    Edit,
    Delete,
    Move,
    Network,
    Push,
    ForcePush,
    PullRequest,
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
        tool == "run" -> ApprovalKind.Command
        tool == "write_file" || tool == "edit_file" || tool == "multi_edit" -> ApprovalKind.Edit
        tool == "delete_file" -> ApprovalKind.Delete
        tool == "move_file" -> ApprovalKind.Move
        tool == "git_push" && target.startsWith(FORCE_PUSH_PREFIX) -> ApprovalKind.ForcePush
        tool == "git_push" -> ApprovalKind.Push
        tool == "pr_create" -> ApprovalKind.PullRequest
        tool.startsWith("web_") -> ApprovalKind.Network
        else -> ApprovalKind.Other
    }

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
