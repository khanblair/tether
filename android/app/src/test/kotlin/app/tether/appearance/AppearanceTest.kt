package app.tether.appearance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AppearanceTest {
    @Test
    fun defaults_are_100_percent_interface_follow_system_text_and_no_accent() {
        val defaults = Appearance()
        assertEquals(100, defaults.interfacePercent)
        assertEquals(TEXT_FOLLOW_SYSTEM, defaults.textPercent)
        assertNull(defaults.accentArgb)
    }

    @Test
    fun every_allowed_value_is_kept_as_it_is() {
        for (interfacePercent in INTERFACE_PERCENTS) {
            for (textPercent in TEXT_PERCENTS) {
                val normalized =
                    Appearance(
                        interfacePercent,
                        textPercent,
                        0xFF112233.toInt(),
                    ).normalized()
                assertEquals(interfacePercent, normalized.interfacePercent)
                assertEquals(textPercent, normalized.textPercent)
            }
        }
    }

    @Test
    fun an_unknown_interface_size_snaps_to_the_nearest_allowed_size() {
        assertEquals(90, Appearance(interfacePercent = 87).normalized().interfacePercent)
        assertEquals(80, Appearance(interfacePercent = 10).normalized().interfacePercent)
        assertEquals(125, Appearance(interfacePercent = 400).normalized().interfacePercent)
    }

    @Test
    fun an_unknown_text_size_becomes_follow_system() {
        assertEquals(TEXT_FOLLOW_SYSTEM, Appearance(textPercent = 77).normalized().textPercent)
    }

    @Test
    fun an_accent_of_zero_is_dropped() {
        assertNull(Appearance(accentArgb = 0).normalized().accentArgb)
    }

    @Test
    fun any_other_accent_is_kept() {
        assertEquals(
            0xFF0066CC.toInt(),
            Appearance(accentArgb = 0xFF0066CC.toInt()).normalized().accentArgb,
        )
    }
}
