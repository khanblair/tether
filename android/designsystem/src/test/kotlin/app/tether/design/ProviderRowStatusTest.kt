package app.tether.design

import app.tether.design.components.ProviderRowStatus
import org.junit.Assert.assertEquals
import org.junit.Test

class ProviderRowStatusTest {
    @Test
    fun statusesAreTheFiveNamesInOrder() {
        assertEquals(
            listOf("Unset", "Ok", "BadKey", "NetworkError", "Blocked"),
            ProviderRowStatus.values().map { it.name },
        )
    }
}
