package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class SessionSheetTextTest {
    @Test
    fun whole_dollar_prices_drop_the_cents() {
        assertEquals("$3", formatPricePerMillion(3_000_000))
    }

    @Test
    fun fractional_prices_keep_two_places_without_trailing_zeros() {
        assertEquals("$1.25", formatPricePerMillion(1_250_000))
        assertEquals("$0.5", formatPricePerMillion(500_000))
    }

    @Test
    fun a_zero_cost_shows_the_unspent_text() {
        assertEquals("none", costLabel(0, unspent = "none"))
    }

    @Test
    fun a_cost_shows_dollars_and_cents() {
        assertEquals("$0.21", costLabel(210_000, unspent = "none"))
    }
}
