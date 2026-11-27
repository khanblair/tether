package app.tether.newsession

import java.util.concurrent.ConcurrentHashMap

/** Holds one first message until the session route has opened its connection. */
class PendingFirstMessage {
    private val messages = ConcurrentHashMap<String, String>()

    /** Replaces the pending message for [sessionId]. */
    fun put(
        sessionId: String,
        text: String,
    ) {
        messages[sessionId] = text
    }

    /** Returns and removes the pending message for [sessionId]. */
    fun take(sessionId: String): String? = messages.remove(sessionId)
}
