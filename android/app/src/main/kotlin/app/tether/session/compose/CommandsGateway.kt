package app.tether.session.compose

import app.tether.core.generated.CommandRuntimeException
import app.tether.core.generated.CommandView
import app.tether.core.generated.SessionRuntime

/**
 * One built-in command as the `/` sheet lists it. [argsHint] is null when the command takes no
 * arguments. [requires] names the missing feature when the command is not [available].
 */
data class ComposerCommand(
    val name: String,
    val description: String,
    val argsHint: String?,
    val available: Boolean,
    val requires: String?,
) {
    /** True when the command takes arguments, so picking it leaves a space after the name. */
    val takesArguments: Boolean get() = argsHint != null
}

/** How the core resolved a typed command. */
sealed interface CommandResolution {
    /** The command ran as [kind]. [prompt] is set for a prompt command. */
    data class Resolved(
        val name: String,
        val arguments: String,
        val kind: String,
        val prompt: String?,
    ) : CommandResolution

    /** No command has this name. [suggestions] are the closest names. */
    data class Unknown(
        val name: String,
        val suggestions: List<String>,
    ) : CommandResolution

    /** The command exists but needs a feature that is off. [reason] is plain text. */
    data class Unavailable(
        val reason: String,
    ) : CommandResolution
}

/**
 * The built-in commands and command resolution from the Rust core. Every call blocks, so callers
 * run it off the main thread. A fake implements this in unit tests.
 */
interface CommandsGateway {
    /** Every built-in command, available or not, in the order the core lists them. */
    fun commands(): List<ComposerCommand>

    /** Resolves a typed command such as `/compact now`. */
    fun resolve(input: String): CommandResolution
}

/**
 * The real gateway over the session runtime of an open connection. The runtime opens on the
 * first call, which runs off the main thread.
 */
class CoreCommandsGateway(
    private val runtime: Lazy<SessionRuntime>,
) : CommandsGateway {
    override fun commands(): List<ComposerCommand> =
        runtime.value.commandSheet().map { it.toCommand() }

    override fun resolve(input: String): CommandResolution =
        try {
            runtime.value.resolveCommand(input).let { view ->
                CommandResolution.Resolved(
                    name = view.name,
                    arguments = view.arguments,
                    kind = view.kind,
                    prompt = view.prompt,
                )
            }
        } catch (error: CommandRuntimeException) {
            error.toResolution()
        }
}

/** The resolution a core error stands for. Each variant maps to one message. */
internal fun CommandRuntimeException.toResolution(): CommandResolution =
    when (this) {
        is CommandRuntimeException.Unknown -> CommandResolution.Unknown(name, suggestions)
        is CommandRuntimeException.Unavailable -> CommandResolution.Unavailable(reason)
    }

private fun CommandView.toCommand(): ComposerCommand =
    ComposerCommand(
        name = name,
        description = description,
        argsHint = argsHint,
        available = available,
        requires = requires,
    )
