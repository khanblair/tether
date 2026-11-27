package app.tether.design

import androidx.compose.ui.graphics.Color
import app.tether.design.tokens.TetherPalette
import kotlin.math.max
import kotlin.math.min

private const val CONTRAST_MINIMUM = 4.5
private const val CHROMA_FLOOR = 0.04
private const val TEXT_CHROMA = 0.025
private const val LIGHT_TEXT_LIGHTNESS = 0.98
private const val DARK_TEXT_LIGHTNESS = 0.02
private const val LIGHT_INK_LIGHTNESS = 0.28
private const val DARK_INK_LIGHTNESS = 0.82
private const val LIGHT_SOFT_LIGHTNESS = 0.92
private const val DARK_SOFT_LIGHTNESS = 0.24
private const val LIGHT_TONAL_LIGHTNESS = 0.76
private const val DARK_TONAL_LIGHTNESS = 0.38
private const val DARK_ACCENT_LIGHTNESS = 0.78
private const val CHROMA_SOFT_FACTOR = 0.35
private const val CHROMA_TONAL_FACTOR = 0.55
private const val HUE_SHIFT = 25.0

// WCAG relative luminance adds this offset to both terms of the ratio.
private const val WCAG_LUMINANCE_OFFSET = 0.05

/** A named color that the appearance picker can display with its own localized name. */
data class AccentSwatch(
    val id: String,
    val seed: Color,
)

/** The built-in accent choices. Display names belong to the app resources. */
val ACCENT_SWATCHES: List<AccentSwatch> =
    listOf(
        AccentSwatch("plum", Color(0xFF64165D)),
        AccentSwatch("blue", Color(0xFF2563EB)),
        AccentSwatch("green", Color(0xFF15803D)),
        AccentSwatch("yellow", Color(0xFFEAB308)),
        AccentSwatch("red", Color(0xFFDC2626)),
    )

/** Returns the WCAG 2.x contrast ratio for two opaque sRGB colors. */
fun contrastRatio(
    foreground: Color,
    background: Color,
): Double {
    val foregroundLuminance = relativeLuminance(foreground)
    val backgroundLuminance = relativeLuminance(background)
    val lighter = max(foregroundLuminance, backgroundLuminance)
    val darker = min(foregroundLuminance, backgroundLuminance)
    return (lighter + WCAG_LUMINANCE_OFFSET) / (darker + WCAG_LUMINANCE_OFFSET)
}

/** Derives the user accent fields while preserving every other value in [base]. */
fun deriveAccentPalette(
    base: TetherPalette,
    seed: Color,
    dark: Boolean,
): TetherPalette {
    if (seed == DEFAULT_PLUM) return base
    val source = seed.toOklch()
    val chroma = max(source.chroma, CHROMA_FLOOR)
    val hue = source.hue
    val accent = deriveAccent(source, chroma, hue, dark)
    val onAccent = accentText(accent, hue, dark)
    val ink = deriveInk(base, hue, chroma, dark)
    val soft = deriveSoft(ink, hue, chroma, dark)
    val tonal = deriveTonal(base, hue, chroma, dark)
    val onTonal = accentText(tonal, hue, forceDark = false)
    val accent2 = deriveAccentTwo(base, accent, hue, chroma, dark)
    val onAccent2 = accentText(accent2, normalizeHue(hue + HUE_SHIFT), dark)
    return base.copy(
        color =
            base.color.copy(
                accent = accent,
                onAccent = onAccent,
                accentInk = ink,
                accentSoft = soft,
                accent2 = accent2,
                onAccent2 = onAccent2,
                tonal = tonal,
                onTonal = onTonal,
            ),
    )
}

private fun deriveAccent(
    source: Oklch,
    chroma: Double,
    hue: Double,
    dark: Boolean,
): Color {
    val lightness = if (dark) max(source.lightness, DARK_ACCENT_LIGHTNESS) else source.lightness
    val candidate = Oklch(lightness, chroma, hue).toColor()
    val text = accentText(candidate, hue, dark)
    return if (contrastRatio(text, candidate) >= CONTRAST_MINIMUM) {
        candidate
    } else {
        ensureContrast(candidate, text, CONTRAST_MINIMUM)
    }
}

private fun deriveInk(
    base: TetherPalette,
    hue: Double,
    chroma: Double,
    dark: Boolean,
): Color {
    val lightness = if (dark) DARK_INK_LIGHTNESS else LIGHT_INK_LIGHTNESS
    var ink = Oklch(lightness, chroma, hue).toColor()
    ink = ensureContrast(ink, base.color.surface, CONTRAST_MINIMUM)
    ink = ensureContrast(ink, base.color.bg, CONTRAST_MINIMUM)
    return ink
}

private fun deriveSoft(
    ink: Color,
    hue: Double,
    chroma: Double,
    dark: Boolean,
): Color {
    val lightness = if (dark) DARK_SOFT_LIGHTNESS else LIGHT_SOFT_LIGHTNESS
    val soft = Oklch(lightness, max(chroma * CHROMA_SOFT_FACTOR, CHROMA_FLOOR), hue).toColor()
    return ensureContrast(soft, ink, CONTRAST_MINIMUM)
}

private fun deriveTonal(
    base: TetherPalette,
    hue: Double,
    chroma: Double,
    dark: Boolean,
): Color {
    val lightness = if (dark) DARK_TONAL_LIGHTNESS else LIGHT_TONAL_LIGHTNESS
    val tonal = Oklch(lightness, max(chroma * CHROMA_TONAL_FACTOR, CHROMA_FLOOR), hue).toColor()
    return ensureContrast(tonal, base.color.text, CONTRAST_MINIMUM)
}

private fun deriveAccentTwo(
    base: TetherPalette,
    accent: Color,
    hue: Double,
    chroma: Double,
    dark: Boolean,
): Color {
    val accentTone = accent.toOklch()
    val secondHue = normalizeHue(hue + HUE_SHIFT)
    val candidate = Oklch(accentTone.lightness, chroma, secondHue).toColor()
    return if (dark) candidate else ensureContrast(candidate, base.color.surface, CONTRAST_MINIMUM)
}

private fun accentText(
    background: Color,
    hue: Double,
    forceDark: Boolean,
): Color {
    val darkText = Oklch(DARK_TEXT_LIGHTNESS, TEXT_CHROMA, hue).toColor()
    val lightText = Oklch(LIGHT_TEXT_LIGHTNESS, TEXT_CHROMA, hue).toColor()
    val selected =
        if (forceDark) {
            darkText
        } else if (contrastRatio(lightText, background) >= contrastRatio(darkText, background)) {
            lightText
        } else {
            darkText
        }
    return ensureContrast(selected, background, CONTRAST_MINIMUM)
}

private val DEFAULT_PLUM = Color(0xFF64165D)
