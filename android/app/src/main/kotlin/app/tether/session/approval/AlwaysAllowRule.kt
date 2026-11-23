package app.tether.session.approval

/** The kind of call an Always allow rule covers. */
enum class AlwaysAllowKind {
    /** Commands that start with [AlwaysAllowRule.value], such as `ls`. */
    Command,

    /** Edits to exactly the file at [AlwaysAllowRule.value]. */
    Edit,
}

/**
 * The rule that an Always allow answer creates for this project. [value] is the program name for
 * [AlwaysAllowKind.Command], or the relative file path for [AlwaysAllowKind.Edit].
 */
data class AlwaysAllowRule(
    val kind: AlwaysAllowKind,
    val value: String,
)

/** Characters that let a command do more than its first word. Such a command gets no rule. */
private const val SHELL_CHARACTERS = ";&|`$<>()\n\\"

/** The label the screens put in front of a command, such as `[linux] `. */
private val LABEL_PREFIX = Regex("""^\[[^\]]*\]\s*""")

/** Tools whose Always allow rule is a file path. `move_file` has no rule. */
private val EDIT_TOOLS = setOf("write_file", "edit_file", "multi_edit", "delete_file")

/**
 * Returns the rule that Always allow creates for a call of [tool] on [target], or null when the
 * call gets no rule. This mirrors the core's rule maker. The core keeps only the program name of
 * a command with no shell characters, and only an exact relative path for a file edit.
 *
 * @param target the display text of the call. A leading `[name] ` label is removed first.
 */
fun alwaysAllowRule(
    tool: String,
    target: String,
): AlwaysAllowRule? =
    when {
        tool == "run" -> commandRule(target)
        tool in EDIT_TOOLS -> editRule(target)
        else -> null
    }

private fun commandRule(target: String): AlwaysAllowRule? {
    val command = target.replace(LABEL_PREFIX, "").trim()
    val program = command.takeWhile { !it.isWhitespace() }
    val safe = program.isNotEmpty() && command.none { it in SHELL_CHARACTERS }
    return if (safe) AlwaysAllowRule(AlwaysAllowKind.Command, program) else null
}

private fun editRule(target: String): AlwaysAllowRule? {
    val path = target.trim()
    val segments = path.split('/').filter { it.isNotEmpty() }
    val safe =
        !path.startsWith("/") &&
            segments.isNotEmpty() &&
            segments.none { it == "." || it == ".." }
    return if (safe) AlwaysAllowRule(AlwaysAllowKind.Edit, path) else null
}
