package app.tether.design

import app.tether.design.components.BadgeTone
import org.junit.Assert.assertEquals
import org.junit.Test

class BadgeToneTest {
    @Test
    fun tonesAreTheFiveNamesInOrder() {
        assertEquals(
            listOf("Neutral", "Tonal", "Success", "Warning", "Danger"),
            BadgeTone.values().map { it.name },
        )
    }
}
