package app.tether.design

import androidx.compose.ui.graphics.Color
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

private const val CONTRAST_MARGIN = 0.01
private const val BISECTION_STEPS = 28
private const val BISECTION_DIVISOR = 2.0
private const val INVERSE_THREE = 1.0 / 3.0
private const val SRGB_THRESHOLD = 0.04045
private const val SRGB_LINEAR_SCALE = 12.92
private const val SRGB_OFFSET = 0.055
private const val SRGB_SCALE = 1.055
private const val SRGB_EXPONENT = 2.4
private const val SRGB_MAX = 1.0
private const val SRGB_MIN = 0.0
private const val DEGREES = 360.0
private const val LUMINANCE_RED = 0.2126
private const val LUMINANCE_GREEN = 0.7152
private const val LUMINANCE_BLUE = 0.0722
private const val MATRIX_L = 0.4122214708
private const val MATRIX_M = 0.5363325363
private const val MATRIX_S = 0.0514459929
private const val MATRIX_M_L = 0.2119034982
private const val MATRIX_M_M = 0.6806995451
private const val MATRIX_M_S = 0.1073969566
private const val MATRIX_S_L = 0.0883024619
private const val MATRIX_S_M = 0.2817188376
private const val MATRIX_S_S = 0.6299787005
private const val OKL_LINEAR_L = 0.2104542553
private const val OKL_LINEAR_M = 0.793617785
private const val OKL_LINEAR_S = -0.0040720468
private const val OKL_A_L = 1.9779984951
private const val OKL_A_M = -2.428592205
private const val OKL_A_S = 0.4505937099
private const val OKL_B_L = 0.0259040371
private const val OKL_B_M = 0.7827717662
private const val OKL_B_S = -0.808675766
private const val OKLCH_L = 0.3963377774
private const val OKLCH_L_B = 0.2158037573
private const val OKLCH_M = -0.1055613458
private const val OKLCH_M_B = -0.0638541728
private const val OKLCH_S = -0.0894841775
private const val OKLCH_S_B = -1.291485548
private const val LINEAR_RED_L = 4.0767416621
private const val LINEAR_RED_M = -3.3077115913
private const val LINEAR_RED_S = 0.2309699292
private const val LINEAR_GREEN_L = -1.2684380046
private const val LINEAR_GREEN_M = 2.6097574011
private const val LINEAR_GREEN_S = -0.3413193965
private const val LINEAR_BLUE_L = -0.0041960863
private const val LINEAR_BLUE_M = -0.7034186147
private const val LINEAR_BLUE_S = 1.707614701

/** A color in the OKLCH cylindrical form. Lightness and chroma are in the 0..1 range. */
data class Oklch(
    val lightness: Double,
    val chroma: Double,
    val hue: Double,
)

private data class LinearRgb(
    val red: Double,
    val green: Double,
    val blue: Double,
)

/** Converts an sRGB color to OKLCH. The color is treated as opaque. */
fun Color.toOklch(): Oklch {
    val red = srgbToLinear(red.toDouble())
    val green = srgbToLinear(green.toDouble())
    val blue = srgbToLinear(blue.toDouble())
    val l = cubeRoot(MATRIX_L * red + MATRIX_M * green + MATRIX_S * blue)
    val m = cubeRoot(MATRIX_M_L * red + MATRIX_M_M * green + MATRIX_M_S * blue)
    val s = cubeRoot(MATRIX_S_L * red + MATRIX_S_M * green + MATRIX_S_S * blue)
    val lightness = OKL_LINEAR_L * l + OKL_LINEAR_M * m + OKL_LINEAR_S * s
    val a = OKL_A_L * l + OKL_A_M * m + OKL_A_S * s
    val b = OKL_B_L * l + OKL_B_M * m + OKL_B_S * s
    return Oklch(
        lightness = lightness,
        chroma = sqrt(a * a + b * b),
        hue = normalizeHue(Math.toDegrees(atan2(b, a))),
    )
}

/** Converts OKLCH to an sRGB color, reducing chroma until the result is in gamut. */
fun Oklch.toColor(): Color {
    val linearRgb = oklchLinearRgb(lightness, chroma, hue)
    return gamutColor(lightness, chroma, hue, linearRgb)
}

/** Moves a color toward the black or white end that gives the best contrast. */
fun ensureContrast(
    color: Color,
    background: Color,
    minimum: Double,
): Color {
    val required = minimum + CONTRAST_MARGIN
    var result = color
    if (contrastRatio(color, background) < required) {
        val source = color.toOklch()
        val darkEnd = Oklch(SRGB_MIN, source.chroma, source.hue).toColor()
        val lightEnd = Oklch(SRGB_MAX, source.chroma, source.hue).toColor()
        val towardDark = contrastRatio(darkEnd, background) >= contrastRatio(lightEnd, background)
        val end = if (towardDark) SRGB_MIN else SRGB_MAX
        val endColor = if (towardDark) darkEnd else lightEnd
        if (contrastRatio(endColor, background) < required) {
            result = endColor
        } else {
            result = findContrastColor(source, background, required, towardDark, end)
        }
    }
    return result
}

private fun findContrastColor(
    source: Oklch,
    background: Color,
    required: Double,
    towardDark: Boolean,
    end: Double,
): Color {
    var low = min(source.lightness, end)
    var high = max(source.lightness, end)
    repeat(BISECTION_STEPS) {
        val middle = (low + high) / BISECTION_DIVISOR
        val candidate = Oklch(middle, source.chroma, source.hue).toColor()
        val passes = contrastRatio(candidate, background) >= required
        if (towardDark == passes) low = middle else high = middle
    }
    return Oklch(if (towardDark) low else high, source.chroma, source.hue).toColor()
}

internal fun relativeLuminance(color: Color): Double {
    val red = srgbToLinear(color.red.toDouble())
    val green = srgbToLinear(color.green.toDouble())
    val blue = srgbToLinear(color.blue.toDouble())
    return LUMINANCE_RED * red + LUMINANCE_GREEN * green + LUMINANCE_BLUE * blue
}

private fun srgbToLinear(channel: Double): Double =
    if (channel <= SRGB_THRESHOLD) {
        channel / SRGB_LINEAR_SCALE
    } else {
        ((channel + SRGB_OFFSET) / SRGB_SCALE).pow(SRGB_EXPONENT)
    }

private fun linearToSrgb(channel: Double): Double =
    if (channel <= SRGB_THRESHOLD / SRGB_LINEAR_SCALE) {
        channel * SRGB_LINEAR_SCALE
    } else {
        SRGB_SCALE * channel.pow(INVERSE_THREE) - SRGB_OFFSET
    }

private fun cubeRoot(value: Double): Double =
    if (value < SRGB_MIN) -(-value).pow(INVERSE_THREE) else value.pow(INVERSE_THREE)

private fun cube(value: Double): Double = value * value * value

private fun gamutColor(
    lightness: Double,
    chroma: Double,
    hue: Double,
    linearRgb: LinearRgb,
): Color {
    if (inGamut(linearRgb)) {
        return rgbColor(linearRgb)
    }
    var low = SRGB_MIN
    var high = max(chroma, SRGB_MIN)
    repeat(BISECTION_STEPS) {
        val middle = (low + high) / BISECTION_DIVISOR
        val candidate = oklchLinearRgb(lightness, middle, hue)
        if (inGamut(candidate)) low = middle else high = middle
    }
    val neutral = oklchLinearRgb(lightness, low, hue)
    return rgbColor(neutral)
}

private fun oklchLinearRgb(
    lightness: Double,
    chroma: Double,
    hue: Double,
): LinearRgb {
    val angle = Math.toRadians(hue)
    val a = chroma * cos(angle)
    val b = chroma * sin(angle)
    val l = lightness + OKLCH_L * a + OKLCH_L_B * b
    val m = lightness + OKLCH_M * a + OKLCH_M_B * b
    val s = lightness + OKLCH_S * a + OKLCH_S_B * b
    return LinearRgb(
        red = LINEAR_RED_L * cube(l) + LINEAR_RED_M * cube(m) + LINEAR_RED_S * cube(s),
        green = LINEAR_GREEN_L * cube(l) + LINEAR_GREEN_M * cube(m) + LINEAR_GREEN_S * cube(s),
        blue = LINEAR_BLUE_L * cube(l) + LINEAR_BLUE_M * cube(m) + LINEAR_BLUE_S * cube(s),
    )
}

private fun inGamut(rgb: LinearRgb): Boolean =
    rgb.red in SRGB_MIN..SRGB_MAX &&
        rgb.green in SRGB_MIN..SRGB_MAX &&
        rgb.blue in SRGB_MIN..SRGB_MAX

private fun rgbColor(rgb: LinearRgb): Color =
    Color(
        linearToSrgb(rgb.red).coerceIn(SRGB_MIN, SRGB_MAX).toFloat(),
        linearToSrgb(rgb.green).coerceIn(SRGB_MIN, SRGB_MAX).toFloat(),
        linearToSrgb(rgb.blue).coerceIn(SRGB_MIN, SRGB_MAX).toFloat(),
    )

internal fun normalizeHue(hue: Double): Double = ((hue % DEGREES + DEGREES) % DEGREES)
