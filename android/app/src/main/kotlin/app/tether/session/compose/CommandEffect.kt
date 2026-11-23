package app.tether.session.compose

/** A screen a slash command opens. Each value names one destination in the session screen. */
enum class CommandScreen {
    Diff,
    Linux,
    CiRuns,
    Cost,
    Context,
    Skills,
    Memory,
    Vault,
    Help,
}

/**
 * What the app does when the person sends a slash command. The core resolves the command; the
 * app runs the effect. [Unsupported] covers commands that are declared but not built yet, and any
 * name this table does not know. A slash command must never reach the model by mistake.
 */
sealed interface CommandEffect {
    /** Sends the finished [text] of a prompt command to the model. */
    data class SendPrompt(
        val text: String,
    ) : CommandEffect

    /** Creates another session in the current project. */
    data object NewSession : CommandEffect

    /** Hides earlier messages from future context. */
    data object ClearContext : CommandEffect

    /** Switches the session to Plan mode. */
    data object SetPlanMode : CommandEffect

    /** Exports the current session as Markdown. */
    data object ExportSession : CommandEffect

    /** Opens the model picker. */
    data object OpenModelSheet : CommandEffect

    /** Opens the permission mode picker. */
    data object OpenModeSheet : CommandEffect

    /** Opens the checkpoint list. */
    data object OpenRewindSheet : CommandEffect

    /**
     * Opens [target]. [arguments] is the trimmed text after the name, such as `lint` or a skill
     * name.
     */
    data class OpenScreen(
        val target: CommandScreen,
        val arguments: String,
    ) : CommandEffect

    /** The command cannot run in the app yet. [name] is the command name without the slash. */
    data class Unsupported(
        val name: String,
    ) : CommandEffect
}
