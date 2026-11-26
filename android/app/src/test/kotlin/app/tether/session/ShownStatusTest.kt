package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class ShownStatusTest {
    private val saved =
        SessionStatus(
            modelId = "sonnet",
            mode = "ask",
            tokensIn = 1,
            tokensOut = 2,
            costMicros = 0,
            contextWindow = null,
        )

    @Test
    fun bypass_on_shows_the_status_line_as_bypass() {
        assertEquals(saved.copy(mode = "bypass"), saved.shownWith(bypass = true))
    }

    @Test
    fun bypass_off_keeps_the_saved_status() {
        assertEquals(saved, saved.shownWith(bypass = false))
    }
}
