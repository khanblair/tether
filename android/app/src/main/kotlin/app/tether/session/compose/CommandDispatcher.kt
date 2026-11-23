package app.tether.session.compose

/** What sending the composer's text does. The caller acts on each case. */
sealed interface SendDecision {
    /** Plain text for the model, sent as a message. */
    data class SendText(
        val text: String,
    ) : SendDecision

    /** A slash command the app runs as [effect]. Nothing is sent to the model. */
    data class Run(
        val effect: CommandEffect,
    ) : SendDecision

    /** Nothing runs. [text] is a message for the snackbar. */
    data class Notice(
        val text: String,
    ) : SendDecision
}

/**
 * Decides what sending the composer's text does: plain text, a command to run, or a notice. It
 * holds no Android resources; the words come in through [CommandWords].
 */
class CommandDispatcher(
    private val gateway: CommandsGateway,
) {
    /**
     * Decides what [text] does. A text that starts with a slash and a name is resolved through the
     * gateway. A lone slash, or a slash followed by a space, is plain text. Every call blocks on
     * the gateway, so callers run it off the main thread.
     */
    fun decide(
        text: String,
        words: CommandWords,
    ): SendDecision =
        if (isCommandText(text)) {
            resolvedDecision(text, words)
        } else {
            SendDecision.SendText(text)
        }

    private fun resolvedDecision(
        text: String,
        words: CommandWords,
    ): SendDecision {
        val resolution = runCatching { gateway.resolve(text) }.getOrNull()
        return if (resolution == null) {
            SendDecision.Notice(words.failed)
        } else {
            decisionFor(resolution, words)
        }
    }

    private fun decisionFor(
        resolution: CommandResolution,
        words: CommandWords,
    ): SendDecision =
        when (resolution) {
            is CommandResolution.Resolved -> runResolved(resolution, words)
            is CommandResolution.Unknown ->
                SendDecision.Notice(
                    words.unknown(resolution.name, resolution.suggestions.firstOrNull()),
                )
            is CommandResolution.Unavailable ->
                SendDecision.Notice(resolution.reason.ifBlank { words.notAvailableYet })
        }

    private fun runResolved(
        resolution: CommandResolution.Resolved,
        words: CommandWords,
    ): SendDecision {
        val effect = commandEffectOf(resolution)
        return if (effect is CommandEffect.Unsupported) {
            SendDecision.Notice(words.notAvailableYet)
        } else {
            SendDecision.Run(effect)
        }
    }
}

/** True when [text] starts with a slash and a name, so it is a command and not plain text. */
internal fun isCommandText(text: String): Boolean {
    if (!text.startsWith("/")) return false
    val rest = text.substring(1)
    return rest.isNotEmpty() && !rest.first().isWhitespace()
}

/**
 * The words the dispatcher can show. [unknown] takes the typed name without the slash and the
 * closest suggestion without the slash, or null when there is none. [notAvailableYet] covers a
 * command the app does not run yet. [failed] covers a gateway that throws.
 */
data class CommandWords(
    val unknown: (name: String, suggestion: String?) -> String,
    val notAvailableYet: String,
    val failed: String,
)
