package app.tether.newsession

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PendingFirstMessageTest {
    @Test
    fun take_returns_text_once() {
        val pending = PendingFirstMessage()
        pending.put("session-1", "Hello")

        assertEquals("Hello", pending.take("session-1"))
        assertNull(pending.take("session-1"))
    }

    @Test
    fun messages_are_keyed_by_session_id() {
        val pending = PendingFirstMessage()
        pending.put("session-1", "One")
        pending.put("session-2", "Two")

        assertEquals("One", pending.take("session-1"))
        assertEquals("Two", pending.take("session-2"))
    }
}
