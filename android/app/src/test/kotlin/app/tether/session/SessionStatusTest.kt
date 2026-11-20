package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SessionStatusTest {
    @Test fun contextIsUnknownWithoutCatalogData() {
        assertNull(status(500, null).contextPercent)
    }

    @Test fun contextPercentIsBounded() {
        assertEquals(50, status(500, 1_000).contextPercent)
        assertEquals(100, status(2_000, 1_000).contextPercent)
    }

    @Test fun costUsesMicros() {
        assertEquals("0.21", status(0, null, 210_000).costText())
    }

    private fun status(
        tokens: Long,
        window: Long?,
        cost: Long = 0,
    ) = SessionStatus("model", "ask", tokens, 0, cost, window)
}
