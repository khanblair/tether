package app.tether.appearance

import app.tether.design.ThemeMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class AppearanceLogicTest {
    @Test
    fun parses_short_long_lowercase_and_spaced_hex_values() {
        assertEquals(HexColorResult.Valid(0xFFAABBCC.toInt()), parseHexColor(" #abc "))
        assertEquals(HexColorResult.Valid(0xFFAABBCC.toInt()), parseHexColor("aAbBcC"))
        assertEquals(HexColorResult.Valid(0xFF112233.toInt()), parseHexColor("#112233"))
    }

    @Test
    fun rejects_empty_wrong_length_and_bad_hex_values() {
        assertEquals(HexColorResult.Invalid(HexError.Empty), parseHexColor("  "))
        assertEquals(HexColorResult.Invalid(HexError.InvalidLength), parseHexColor("#12"))
        assertEquals(HexColorResult.Invalid(HexError.InvalidCharacters), parseHexColor("#12xz45"))
    }

    @Test
    fun hsv_round_trip_keeps_hue_saturation_and_value_within_one_channel() {
        val colors = listOf(0xFF64165D.toInt(), 0xFF2563EB.toInt(), 0xFFEAB308.toInt())

        colors.forEach { color ->
            val hsv = argbToHsv(color)
            val roundTrip = hsvToArgb(hsv.hue, hsv.saturation, hsv.value)
            assertTrue(channelDistance(color, roundTrip) <= 1)
        }
    }

    @Test
    fun summary_lists_theme_text_and_only_non_default_interface_size() {
        val labels = summaryLabels()

        summaryCases().forEach { case ->
            val summary =
                appearanceRowSummary(
                    case.mode,
                    Appearance(case.interfacePercent, case.textPercent),
                    labels,
                )
            assertSummary(summary, case)
        }
    }

    @Test
    fun exposes_the_decided_steps_and_labels() {
        assertEquals(listOf(80, 90, 100, 110, 125), interfaceSizeSteps())
        assertEquals(listOf(80, 90, 100, 115, 130, 150), manualTextSizeSteps())
        assertEquals("115%", sizeStepLabel(115))
    }

    @Test
    fun selects_a_built_in_swatch_when_the_argb_matches_its_seed() {
        val choices =
            listOf(
                AccentChoice("plum", 0xFF64165D.toInt()),
                AccentChoice("blue", 0xFF2563EB.toInt()),
            )

        assertEquals("blue", selectedAccentId(0xFF2563EB.toInt(), choices))
        assertEquals("plum", selectedAccentId(0xFF64165D.toInt(), choices))
        assertEquals(null, selectedAccentId(0xFF123456.toInt(), choices))
    }

    @Test
    fun formats_a_color_as_the_picker_hex_value() {
        assertEquals("#AABBCC", formatHexColor(0xFFAABBCC.toInt()))
    }

    private data class SummaryCase(
        val mode: ThemeMode,
        val textPercent: Int,
        val interfacePercent: Int,
    )

    private fun summaryLabels() =
        AppearanceSummaryLabels(
            theme = { mode -> mode.name },
            followsSystem = "text follows system",
            textPercent = { percent -> "text $percent%" },
            interfacePercent = { percent -> "interface $percent%" },
        )

    private fun summaryCases(): List<SummaryCase> = ThemeMode.entries.flatMap(::summaryCasesForMode)

    private fun summaryCasesForMode(mode: ThemeMode): List<SummaryCase> =
        TEXT_PERCENTS.flatMap { textPercent -> summaryCasesForText(mode, textPercent) }

    private fun summaryCasesForText(
        mode: ThemeMode,
        textPercent: Int,
    ): List<SummaryCase> =
        INTERFACE_PERCENTS.map { interfacePercent ->
            SummaryCase(mode, textPercent, interfacePercent)
        }

    private fun assertSummary(
        summary: String,
        case: SummaryCase,
    ) {
        assertTrue(summary.startsWith(case.mode.name))
        val textLabel =
            if (case.textPercent == 0) "text follows system" else "text ${case.textPercent}%"
        assertTrue(summary.contains(textLabel))
        if (case.interfacePercent == 100) {
            assertTrue(!summary.contains("interface"))
        } else {
            assertTrue(summary.contains("interface ${case.interfacePercent}%"))
        }
    }

    private fun channelDistance(
        first: Int,
        second: Int,
    ): Int {
        val red = kotlin.math.abs(((first shr 16) and 0xFF) - ((second shr 16) and 0xFF))
        val green = kotlin.math.abs(((first shr 8) and 0xFF) - ((second shr 8) and 0xFF))
        val blue = kotlin.math.abs((first and 0xFF) - (second and 0xFF))
        return maxOf(red, green, blue)
    }
}
