package app.tether.design

import app.tether.design.components.SessionState
import org.junit.Assert.assertEquals
import org.junit.Test

class SessionStateTest {
    @Test
    fun statesAreTheFiveNamesInOrder() {
        assertEquals(
            listOf("Running", "Done", "Waiting", "Failed", "Idle"),
            SessionState.values().map { it.name },
        )
    }
}
