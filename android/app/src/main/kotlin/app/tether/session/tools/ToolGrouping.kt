package app.tether.session.tools

import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus

/** Tools that only read. Consecutive calls of these can collapse into one group. */
val READ_ONLY_TOOLS: Set<String> =
    setOf(
        "read_file",
        "glob",
        "grep",
        "list_dir",
        "git_status",
        "git_diff",
        "git_log",
        "pr_status",
        "ci_logs",
    )

/**
 * Statuses that keep a call out of a group, so it stays visible on its own. A call that waits for
 * the user, has not started, runs, or ended in a danger tone or a denial is shown as its own card.
 */
private val GROUP_BREAKING: Set<ToolStatus> =
    setOf(
        ToolStatus.Proposed,
        ToolStatus.Allowed,
        ToolStatus.AwaitingApproval,
        ToolStatus.Running,
        ToolStatus.Failed,
        ToolStatus.TimedOut,
        ToolStatus.Blocked,
        ToolStatus.Denied,
    )

/** One row of the transcript: a single call, or a collapsed group of two or more reads. */
sealed interface CallRow {
    /** One call, shown as its own card. */
    data class Single(
        val call: ToolCallUi,
    ) : CallRow

    /** Two or more consecutive read-only calls, shown as one collapsed card. */
    data class Group(
        val calls: List<ToolCallUi>,
    ) : CallRow
}

/**
 * Collapses runs of consecutive read-only calls into [CallRow.Group]s. A run of fewer than two
 * calls stays as single rows. Any other call ends the run. The order of the calls is kept.
 */
fun groupCalls(calls: List<ToolCallUi>): List<CallRow> {
    val rows = mutableListOf<CallRow>()
    val run = mutableListOf<ToolCallUi>()

    fun flush() {
        if (run.size >= MIN_GROUP_SIZE) {
            rows += CallRow.Group(run.toList())
        } else {
            run.forEach { rows += CallRow.Single(it) }
        }
        run.clear()
    }

    for (call in calls) {
        if (call.isGroupable()) {
            run += call
        } else {
            flush()
            rows += CallRow.Single(call)
        }
    }
    flush()
    return rows
}

private fun ToolCallUi.isGroupable(): Boolean = tool in READ_ONLY_TOOLS && status !in GROUP_BREAKING

/** The smallest run that collapses into a group. */
const val MIN_GROUP_SIZE = 2

/**
 * The counts behind a group summary. [files] are reads, [folders] are directory listings,
 * [searches] are glob and grep, and [checks] are git, pull request, and CI reads.
 */
data class GroupSummary(
    val files: Int,
    val folders: Int,
    val searches: Int,
    val checks: Int,
)

/** Counts the calls of a group by what they did. */
fun groupSummary(calls: List<ToolCallUi>): GroupSummary {
    fun count(vararg tools: String): Int = calls.count { it.tool in tools }
    return GroupSummary(
        files = count("read_file"),
        folders = count("list_dir"),
        searches = count("glob", "grep"),
        checks = count("git_status", "git_diff", "git_log", "pr_status", "ci_logs"),
    )
}

/** The parts of a group summary, in reading order. */
enum class GroupPart {
    Files,
    Folders,
    Searches,
    Checks,
}

/** The parts of [summary] that are not zero, with their counts. */
fun GroupSummary.parts(): List<Pair<GroupPart, Int>> =
    listOf(
        GroupPart.Files to files,
        GroupPart.Folders to folders,
        GroupPart.Searches to searches,
        GroupPart.Checks to checks,
    ).filter { it.second > 0 }
