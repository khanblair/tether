package app.tether.design

import androidx.compose.ui.graphics.Color
import app.tether.design.tokens.TetherPalette
import app.tether.design.tokens.TetherTokens
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class AccentPaletteTest {
    private val seeds =
        listOf(
            Color(0xFF64165D),
            Color(0xFF2563EB),
            Color(0xFF15803D),
            Color(0xFFEAB308),
            Color(0xFFDC2626),
            Color(0xFFF97316),
            Color(0xFF06B6D4),
            Color(0xFF9333EA),
            Color(0xFFEC4899),
            Color.White,
            Color.Black,
            Color(0xFF808080),
            Color.Yellow,
            Color(0xFF0A0A3C),
        )

    @Test
    fun every_seed_produces_readable_light_and_dark_accent_pairs() {
        for (seed in seeds) {
            assertReadable(TetherTokens.light, deriveAccentPalette(TetherTokens.light, seed, false))
            assertReadable(TetherTokens.dark, deriveAccentPalette(TetherTokens.dark, seed, true))
        }
    }

    @Test
    fun derived_palette_preserves_all_non_accent_values() {
        for (seed in seeds.drop(1)) {
            assertNonAccentFieldsEqual(
                TetherTokens.light,
                deriveAccentPalette(TetherTokens.light, seed, false),
            )
            assertNonAccentFieldsEqual(
                TetherTokens.dark,
                deriveAccentPalette(TetherTokens.dark, seed, true),
            )
        }
    }

    @Test
    fun plum_returns_the_original_palette() {
        assertEquals(
            TetherTokens.light,
            deriveAccentPalette(TetherTokens.light, Color(0xFF64165D), false),
        )
        assertEquals(
            TetherTokens.dark,
            deriveAccentPalette(TetherTokens.dark, Color(0xFF64165D), true),
        )
    }

    @Test
    fun derivation_is_deterministic() {
        val first = deriveAccentPalette(TetherTokens.light, Color(0xFF9333EA), false)
        val second = deriveAccentPalette(TetherTokens.light, Color(0xFF9333EA), false)

        assertEquals(first, second)
    }

    @Test
    fun contrast_ratio_has_wcag_end_points() {
        assertEquals(21.0, contrastRatio(Color.White, Color.Black), 0.000001)
        assertEquals(1.0, contrastRatio(Color(0xFF336699), Color(0xFF336699)), 0.000001)
    }

    private fun assertReadable(
        base: TetherPalette,
        palette: TetherPalette,
    ) {
        val colors = palette.color
        assertAtLeast(colors.onAccent, colors.accent, MINIMUM_CONTRAST)
        assertAtLeast(colors.accentInk, colors.surface, MINIMUM_CONTRAST)
        assertAtLeast(colors.accentInk, colors.bg, MINIMUM_CONTRAST)
        assertAtLeast(colors.accentInk, colors.accentSoft, MINIMUM_CONTRAST)
        assertAtLeast(colors.accentInk, colors.surface, FOCUS_CONTRAST)
        assertAtLeast(colors.accent2, colors.surface, MINIMUM_CONTRAST)
        assertAtLeast(colors.onAccent2, colors.accent2, MINIMUM_CONTRAST)
        assertAtLeast(colors.onTonal, colors.tonal, MINIMUM_CONTRAST)
        assertAtLeast(colors.tonal, base.color.text, MINIMUM_CONTRAST)
    }

    private fun assertAtLeast(
        foreground: Color,
        background: Color,
        minimum: Double,
    ) {
        assertTrue(
            "${contrastRatio(foreground, background)} is below $minimum",
            contrastRatio(foreground, background) >= minimum,
        )
    }

    private fun assertNonAccentFieldsEqual(
        base: TetherPalette,
        derived: TetherPalette,
    ) {
        assertEquals(
            base.color.copy(
                accent = derived.color.accent,
                onAccent = derived.color.onAccent,
                accentInk = derived.color.accentInk,
                accentSoft = derived.color.accentSoft,
                accent2 = derived.color.accent2,
                onAccent2 = derived.color.onAccent2,
                tonal = derived.color.tonal,
                onTonal = derived.color.onTonal,
            ),
            derived.color,
        )
        assertEquals(base.syntax, derived.syntax)
        assertEquals(base.chart, derived.chart)
    }

    private companion object {
        const val MINIMUM_CONTRAST = 4.5
        const val FOCUS_CONTRAST = 3.0
    }
}
