package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import app.tether.core.generated.MessageView
import app.tether.session.agent.AgentScreenUi

/** True while a turn runs or an approval waits. Edit and Regenerate are off then. */
internal fun sessionBusy(
    turn: TurnState,
    agent: AgentScreenUi?,
): Boolean = turn.running || agent?.approvalsWaiting == true

/**
 * The Edit and Regenerate actions of one session screen. While [busy] no action is offered, and an
 * open edit sheet closes. [onStart] and [onFailed] are the screen's turn state hooks.
 */
@Composable
internal fun rememberChatActions(
    connection: SessionConnection?,
    messages: List<MessageView>,
    busy: Boolean,
    onStart: () -> Unit,
    onFailed: () -> Unit,
): ChatActionsHandle {
    val scope = rememberCoroutineScope()
    val busyNow by rememberUpdatedState(busy)
    val startNow by rememberUpdatedState(onStart)
    val failedNow by rememberUpdatedState(onFailed)
    val actions =
        remember(connection, scope) {
            connection?.let { active ->
                ChatActions(
                    scope = scope,
                    rewinder = active.rewinder,
                    send = { text -> active.send(text) },
                    busy = { busyNow },
                    hooks = ChatTurnHooks(onStart = { startNow() }, onFailed = { failedNow() }),
                )
            }
        }
    var editingId by rememberSaveable { mutableStateOf<String?>(null) }
    LaunchedEffect(busy) { if (busy) editingId = null }
    val editing = if (busy) null else messages.firstOrNull { it.id == editingId }
    val idle = if (busy) null else actions
    val onEdit: ((MessageView) -> Unit)? =
        if (idle == null) null else ({ message: MessageView -> editingId = message.id })
    val onRegenerate: (() -> Unit)? =
        if (idle == null || lastUserText(messages) == null) {
            null
        } else {
            { idle.regenerate(messages) }
        }
    val onSendEdit = { text: String ->
        val chosen = editing
        if (chosen != null && actions?.edit(chosen, text) == true) editingId = null
    }
    return ChatActionsHandle(
        onEditMessage = onEdit,
        onRegenerate = onRegenerate,
        editing = editing,
        onSendEdit = onSendEdit,
        onDismissEdit = { editingId = null },
    )
}

/** Shows the edit sheet of the message being edited. Nothing shows when there is no edit. */
@Composable
internal fun ChatEditSlot(chat: ChatActionsHandle) {
    chat.editing?.let { message ->
        EditMessageSheet(
            message = message,
            onSend = chat.onSendEdit,
            onDismiss = chat.onDismissEdit,
        )
    }
}
