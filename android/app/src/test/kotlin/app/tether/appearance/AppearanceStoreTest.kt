package app.tether.appearance

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AppearanceStoreTest {
    /** The raw values PrefsAppearanceStore would keep for [appearance]. */
    private fun rawValuesFor(appearance: Appearance): Map<String, Any?> =
        buildMap {
            put("interface_percent", appearance.interfacePercent)
            put("text_percent", appearance.textPercent)
            appearance.accentArgb?.let { put("accent", it) }
        }

    @Test
    fun a_store_with_nothing_saved_loads_the_defaults() {
        assertEquals(Appearance(), appearanceFromValues { null })
    }

    @Test
    fun every_allowed_interface_and_text_value_round_trips() {
        for (interfacePercent in INTERFACE_PERCENTS) {
            for (textPercent in TEXT_PERCENTS) {
                val saved =
                    Appearance(interfacePercent, textPercent, accentArgb = 0xFFAABBCC.toInt())
                val raw = rawValuesFor(saved)
                assertEquals(saved, appearanceFromValues { raw[it] })
            }
        }
    }

    @Test
    fun a_missing_accent_loads_as_no_accent() {
        val raw = rawValuesFor(Appearance(interfacePercent = 110, textPercent = 130))
        val loaded = appearanceFromValues { raw[it] }
        assertNull(loaded.accentArgb)
        assertEquals(110, loaded.interfacePercent)
        assertEquals(130, loaded.textPercent)
    }

    @Test
    fun a_corrupt_store_loads_the_defaults() {
        val corrupt =
            mapOf<String, Any?>(
                "interface_percent" to "huge",
                "text_percent" to 1.5f,
                "accent" to "blue",
            )
        assertEquals(Appearance(), appearanceFromValues { corrupt[it] })
    }

    @Test
    fun a_thrown_preference_read_loads_the_defaults() {
        assertEquals(
            Appearance(),
            appearanceFromValues { throw IllegalStateException("corrupt preferences") },
        )
    }

    @Test
    fun a_stored_unknown_number_snaps_when_loaded() {
        val raw =
            mapOf<String, Any?>("interface_percent" to 87, "text_percent" to 77, "accent" to 0)
        assertEquals(Appearance(interfacePercent = 90), appearanceFromValues { raw[it] })
    }
}
