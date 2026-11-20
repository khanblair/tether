package app.tether.messages

import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/** One message for the snackbar host. An [actionLabel] makes it a snackbar with an action. */
data class AppMessage(
    val text: String,
    val actionLabel: String? = null,
    val onAction: (() -> Unit)? = null,
)

/** Sends messages from anywhere in the app to the one snackbar host. */
class AppMessages {
    private val events = MutableSharedFlow<AppMessage>(extraBufferCapacity = BUFFER)

    /** The messages the host shows, in the order they were sent. */
    val flow: SharedFlow<AppMessage> = events.asSharedFlow()

    /** Sends a message. It is dropped when the buffer is full. */
    fun show(
        text: String,
        actionLabel: String? = null,
        onAction: (() -> Unit)? = null,
    ) {
        events.tryEmit(AppMessage(text, actionLabel, onAction))
    }

    private companion object {
        const val BUFFER = 8
    }
}
