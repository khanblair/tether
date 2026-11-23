package app.tether.session.tools

import app.tether.session.agent.ToolStatus

/** Where a `run` call executes. The core writes the label in front of the command. */
enum class RunExecutor {
    Sandbox,
    Linux,
    Ci,
}

/**
 * A `run` target split into its executor and its command. [executor] is null when the target
 * has no leading label.
 */
data class RunTarget(
    val executor: RunExecutor?,
    val command: String,
)

/** Lines of live output a run card shows while it is collapsed. */
const val RUN_TAIL_LINES = 12

private val EXECUTOR_LABELS: Map<String, RunExecutor> =
    mapOf(
        "[sandbox] " to RunExecutor.Sandbox,
        "[linux] " to RunExecutor.Linux,
        "[ci] " to RunExecutor.Ci,
    )

/**
 * Reads the executor label at the start of a `run` target. Only a label at the very start counts:
 * `[linux] ls` has one, and `echo [linux] ls` does not. A bare `[linux]` with no command is not a
 * label either.
 */
fun parseRunTarget(target: String): RunTarget {
    for ((label, executor) in EXECUTOR_LABELS) {
        if (target.startsWith(label)) {
            return RunTarget(executor, target.substring(label.length))
        }
    }
    return RunTarget(null, target)
}

/** The last [keep] lines of [output], and how many earlier lines were left out. */
data class TailLines(
    val lines: List<String>,
    val hidden: Int,
)

/**
 * The newest [keep] lines of run output. One trailing newline does not make an extra empty line.
 * Empty output gives no lines and nothing hidden.
 */
fun tailLines(
    output: String,
    keep: Int = RUN_TAIL_LINES,
): TailLines {
    val text = output.removeSuffix("\n")
    if (text.isEmpty()) return TailLines(emptyList(), 0)
    val all = text.split("\n")
    val shown = all.takeLast(keep)
    return TailLines(shown, all.size - shown.size)
}

/**
 * True when a run can be retried on another executor: a sandbox run that failed or timed out.
 * Other runs never offer it.
 */
fun canRunElsewhere(
    status: ToolStatus,
    executor: RunExecutor?,
): Boolean =
    executor == RunExecutor.Sandbox &&
        (
            status == ToolStatus.Failed ||
                status == ToolStatus.TimedOut
        )
