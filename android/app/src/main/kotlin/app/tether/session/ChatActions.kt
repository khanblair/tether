package app.tether.session

import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** What the screen does when a chat action starts, and when the core refuses it. */
class ChatTurnHooks(
    val onStart: () -> Unit,
    val onFailed: () -> Unit,
)

/**
 * Edit a user message, and regenerate the newest reply. Each action rewinds the conversation to
 * one message, then sends text as a new turn. The rewind runs first, so a failed rewind sends
 * nothing. [busy] is read at each call: while it is true, both actions refuse and do nothing.
 */
class ChatActions(
    private val scope: CoroutineScope,
    private val rewinder: SessionRewinder,
    private val send: (String) -> Unit,
    private val busy: () -> Boolean,
    private val hooks: ChatTurnHooks,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    /** Rewinds to [message], then sends [newText]. False when refused: busy, or blank text. */
    fun edit(
        message: MessageView,
        newText: String,
    ): Boolean = runTurn(message.id, newText)

    /**
     * Rewinds to the newest user message in [messages], then sends its text again. False when
     * refused: busy, no user message, or a blank message.
     */
    fun regenerate(messages: List<MessageView>): Boolean {
        val newest = messages.lastOrNull { it.role == RoleView.USER } ?: return false
        return runTurn(newest.id, plainText(newest))
    }

    /** Refuses when busy or blank. Otherwise runs the rewind and the send off the main thread. */
    private fun runTurn(
        messageId: String,
        text: String,
    ): Boolean {
        val trimmed = text.trim()
        if (busy() || trimmed.isEmpty()) return false
        hooks.onStart()
        scope.launch {
            try {
                withContext(io) {
                    rewinder.rewindToMessage(messageId)
                    send(trimmed)
                }
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                hooks.onFailed()
            }
        }
        return true
    }
}

/**
 * What a session screen needs for Edit and Regenerate. A null callback hides its action. [editing]
 * is the message whose edit sheet is open, or null.
 */
internal class ChatActionsHandle(
    val onEditMessage: ((MessageView) -> Unit)?,
    val onRegenerate: (() -> Unit)?,
    val editing: MessageView?,
    val onSendEdit: (String) -> Unit,
    val onDismissEdit: () -> Unit,
)
