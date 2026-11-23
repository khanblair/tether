package app.tether.session.tools

import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus

/** What a tool call does, in words people know. [Unknown] covers any tool not listed here. */
enum class ToolTitleKind {
    ReadFile,
    WriteFile,
    EditFile,
    DeleteFile,
    ListDir,
    Search,
    Run,
    CiRun,
    CiLogs,
    GitStatus,
    GitDiff,
    GitLog,
    GitCommit,
    GitPush,
    GitBranch,
    PrCreate,
    PrStatus,
    Unknown,
}

/**
 * The verb form of a title. [Imperative] is for a call the user may still act on, [Present] is for
 * a call that runs, and [Past] is for a call that is over.
 */
enum class ToolTense {
    Imperative,
    Present,
    Past,
}

/** A title: the kind of action, its tense, and the object it acts on (may be empty). */
data class ToolTitle(
    val kind: ToolTitleKind,
    val tense: ToolTense,
    val target: String,
    val tool: String,
)

private val TITLE_KINDS: Map<String, ToolTitleKind> =
    mapOf(
        "read_file" to ToolTitleKind.ReadFile,
        "write_file" to ToolTitleKind.WriteFile,
        "edit_file" to ToolTitleKind.EditFile,
        "delete_file" to ToolTitleKind.DeleteFile,
        "list_dir" to ToolTitleKind.ListDir,
        "glob" to ToolTitleKind.Search,
        "grep" to ToolTitleKind.Search,
        "run" to ToolTitleKind.Run,
        "ci_run" to ToolTitleKind.CiRun,
        "ci_logs" to ToolTitleKind.CiLogs,
        "git_status" to ToolTitleKind.GitStatus,
        "git_diff" to ToolTitleKind.GitDiff,
        "git_log" to ToolTitleKind.GitLog,
        "git_commit" to ToolTitleKind.GitCommit,
        "git_push" to ToolTitleKind.GitPush,
        "git_branch" to ToolTitleKind.GitBranch,
        "pr_create" to ToolTitleKind.PrCreate,
        "pr_status" to ToolTitleKind.PrStatus,
    )

/** The kind of action for a core tool name. An unknown name gives [ToolTitleKind.Unknown]. */
fun toolTitleKind(tool: String): ToolTitleKind = TITLE_KINDS[tool] ?: ToolTitleKind.Unknown

/**
 * The tense for a status: imperative while the user can still act (proposed or allowed, and
 * awaiting approval), present while it runs, and past once it is over.
 */
fun toolTense(status: ToolStatus): ToolTense =
    when (status) {
        ToolStatus.Proposed, ToolStatus.Allowed, ToolStatus.AwaitingApproval -> ToolTense.Imperative
        ToolStatus.Running -> ToolTense.Present
        ToolStatus.Succeeded,
        ToolStatus.Failed,
        ToolStatus.TimedOut,
        ToolStatus.Denied,
        ToolStatus.Blocked,
        ToolStatus.Cancelled,
        -> ToolTense.Past
    }

/**
 * The title of one call. For a `run` call the executor label is removed, because the run card
 * shows the executor as its own badge.
 */
fun toolTitle(call: ToolCallUi): ToolTitle {
    val kind = toolTitleKind(call.tool)
    val target =
        if (kind == ToolTitleKind.Run) {
            parseRunTarget(call.target).command
        } else {
            call.target
        }
    return ToolTitle(kind, toolTense(call.status), target, call.tool)
}
