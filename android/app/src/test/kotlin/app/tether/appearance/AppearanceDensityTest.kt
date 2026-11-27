package app.tether.appearance

import androidx.compose.ui.unit.Density
import org.junit.Assert.assertEquals
import org.junit.Test

class AppearanceDensityTest {
    private val base = Density(density = 2.75f, fontScale = 1.3f)
    private val delta = 0.0001f

    @Test
    fun the_defaults_leave_density_and_font_scale_unchanged() {
        val result = effectiveDensity(base, Appearance(100, TEXT_FOLLOW_SYSTEM))
        assertEquals(2.75f, result.density, delta)
        assertEquals(1.3f, result.fontScale, delta)
    }

    @Test
    fun a_125_percent_interface_scales_density_and_keeps_the_system_font_scale() {
        val result = effectiveDensity(base, Appearance(125, TEXT_FOLLOW_SYSTEM))
        assertEquals(2.75f * 1.25f, result.density, delta)
        assertEquals(1.3f, result.fontScale, delta)
    }

    @Test
    fun a_150_percent_text_size_replaces_the_system_font_scale() {
        val result = effectiveDensity(base, Appearance(100, 150))
        assertEquals(2.75f, result.density, delta)
        assertEquals(1.5f, result.fontScale, delta)
    }

    @Test
    fun an_80_percent_interface_with_80_percent_text_scales_both() {
        val result = effectiveDensity(base, Appearance(80, 80))
        assertEquals(2.75f * 0.8f, result.density, delta)
        assertEquals(0.8f, result.fontScale, delta)
    }
}
