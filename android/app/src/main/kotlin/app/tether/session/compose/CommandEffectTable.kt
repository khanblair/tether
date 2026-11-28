package app.tether.session.compose

import app.tether.session.compose.CommandEffect.ClearContext
import app.tether.session.compose.CommandEffect.ExportSession
import app.tether.session.compose.CommandEffect.NewSession
import app.tether.session.compose.CommandEffect.OpenModeSheet
import app.tether.session.compose.CommandEffect.OpenModelSheet
import app.tether.session.compose.CommandEffect.OpenRewindSheet
import app.tether.session.compose.CommandEffect.OpenScreen
import app.tether.session.compose.CommandEffect.SendPrompt
import app.tether.session.compose.CommandEffect.SetPlanMode
import app.tether.session.compose.CommandEffect.Unsupported

/** The `kind` strings the core sends for each command kind. */
private const val KIND_ACTION = "action"
private const val KIND_OPEN = "open"
private const val KIND_PROMPT = "prompt"
private const val KIND_SKILL = "skill"

/**
 * One row of the command table. [kind] is the core kind the command must have. [effect] builds
 * the effect from the resolved command.
 */
private class CommandRow(
    val kind: String,
    val effect: (CommandResolution.Resolved) -> CommandEffect,
)

private fun action(effect: CommandEffect) = CommandRow(KIND_ACTION) { effect }

private fun open(effect: CommandEffect) = CommandRow(KIND_OPEN) { effect }

private fun openScreen(target: CommandScreen) =
    CommandRow(KIND_OPEN) { resolved -> OpenScreen(target, resolved.arguments) }

private val prompt =
    CommandRow(KIND_PROMPT) { resolved ->
        resolved.prompt?.let { SendPrompt(it) } ?: Unsupported(resolved.name)
    }

/**
 * Every command the app runs itself, keyed by name. `run` and `compact` are left out on purpose:
 * the core declares them for a later phase, so they fall to [Unsupported].
 */
private val commandTable: Map<String, CommandRow> =
    mapOf(
        "new" to action(NewSession),
        "clear" to action(ClearContext),
        "plan" to action(SetPlanMode),
        "export" to action(ExportSession),
        "model" to open(OpenModelSheet),
        "mode" to open(OpenModeSheet),
        "rewind" to open(OpenRewindSheet),
        "diff" to openScreen(CommandScreen.Diff),
        "linux" to openScreen(CommandScreen.Linux),
        "ci" to openScreen(CommandScreen.CiRuns),
        "cost" to openScreen(CommandScreen.Cost),
        "context" to openScreen(CommandScreen.Context),
        "skills" to openScreen(CommandScreen.Skills),
        "memory" to openScreen(CommandScreen.Memory),
        "vault" to openScreen(CommandScreen.Vault),
        "help" to openScreen(CommandScreen.Help),
        "commit" to prompt,
        "push" to prompt,
        "pr" to prompt,
        "review" to prompt,
        "init" to prompt,
    )

/**
 * Maps a resolved core command to the effect the app runs. A name the table does not know, or a
 * core kind that does not match the table, gives [Unsupported]; an unknown name is never sent as
 * a prompt. The `skill` kind is the one exception (see [skillEffect]). A prompt command without
 * its prompt text also gives [Unsupported].
 */
fun commandEffectOf(resolved: CommandResolution.Resolved): CommandEffect {
    if (resolved.kind == KIND_SKILL) return skillEffect(resolved)
    val row = commandTable[resolved.name]
    return if (row != null && row.kind == resolved.kind) {
        row.effect(resolved)
    } else {
        Unsupported(resolved.name)
    }
}

/**
 * A skill command, such as `/tdd fix the bug`. Only the core's installed skills have this kind, so
 * the name is not checked against the table; a project command file never gets here. The prompt
 * text tells the agent to load the skill. Without it the command is [Unsupported].
 */
private fun skillEffect(resolved: CommandResolution.Resolved): CommandEffect =
    resolved.prompt?.let { SendPrompt(it) } ?: Unsupported(resolved.name)
