package app.tether.session

import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView

/**
 * The turn anchors of [messages], keyed by the id of the assistant message that carries them.
 *
 * A turn is a run of assistant and tool messages. A user message or a system note ends a run.
 * The anchor sits on the last assistant message of each run that has one. The run that reaches
 * the end of the list is the newest turn. While [running] is true, that open run gets no anchor.
 */
internal fun turnAnchors(
    messages: List<MessageView>,
    running: Boolean,
): Map<String, TurnAnchor> {
    val anchors = mutableMapOf<String, TurnAnchor>()
    var run = mutableListOf<MessageView>()
    messages.forEach { message ->
        when (message.role) {
            RoleView.ASSISTANT, RoleView.TOOL -> run += message
            RoleView.USER, RoleView.SYSTEM_NOTE -> {
                addAnchor(anchors, run, newest = false)
                run = mutableListOf()
            }
        }
    }
    if (!running) addAnchor(anchors, run, newest = true)
    return anchors
}

private fun addAnchor(
    anchors: MutableMap<String, TurnAnchor>,
    run: List<MessageView>,
    newest: Boolean,
) {
    val last = run.lastOrNull { it.role == RoleView.ASSISTANT } ?: return
    val replyText =
        run
            .filter { it.role == RoleView.ASSISTANT }
            .map(::plainText)
            .filter(String::isNotBlank)
            .joinToString(separator = "\n\n")
    anchors[last.id] = TurnAnchor(replyText, newest)
}
