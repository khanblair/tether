package app.tether.session

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect

/**
 * Sends [initialMessage] once for each [connection] and message pair, through [send], then calls
 * [onStarted]. Nothing is sent while either value is null.
 */
@Composable
internal fun SendInitialMessage(
    connection: SessionConnection?,
    initialMessage: String?,
    send: (SessionConnection, String) -> Unit,
    onStarted: () -> Unit,
) {
    LaunchedEffect(connection, initialMessage) {
        val active = connection
        val text = initialMessage
        if (active != null && text != null) {
            send(active, text)
            onStarted()
        }
    }
}
