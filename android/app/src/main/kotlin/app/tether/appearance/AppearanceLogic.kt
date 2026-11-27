package app.tether.appearance

import app.tether.design.ThemeMode
import kotlin.math.abs
import kotlin.math.roundToInt

private const val FULL_PERCENT = 100
private const val RGB_MAX = 255
private const val HEX_SHORT_LENGTH = 3
private const val HEX_LONG_LENGTH = 6
private const val HEX_RADIX = 16
private const val HUE_MAX = 360.0
private const val HUE_SECTOR = 60.0
private const val HUE_SECTORS = 6
private const val HUE_GREEN_OFFSET = 2.0
private const val HUE_BLUE_OFFSET = 4.0
private const val ALPHA_MASK = 0xFF000000.toInt()
private const val RED_SHIFT = 16
private const val GREEN_SHIFT = 8
private const val CHANNEL_MASK = 0xFF
private const val RGB_MASK = 0x00FFFFFF
private const val HSV_MIN = 0f
private const val HSV_MAX = 1f

// Hex channel positions and width.
private const val HEX_CHANNEL_LENGTH = 2
private const val HEX_RED_START = 0
private const val HEX_GREEN_START = HEX_CHANNEL_LENGTH
private const val HEX_BLUE_START = HEX_CHANNEL_LENGTH * 2
private const val HEX_COLOR_END = HEX_CHANNEL_LENGTH * 3

// HSV sectors, ordered from red through blue.
private const val RED_SECTOR = 0
private const val YELLOW_SECTOR = 1
private const val GREEN_SECTOR = 2
private const val CYAN_SECTOR = 3
private const val BLUE_SECTOR = 4

/** The labels needed to build the main Settings row without putting UI copy in logic. */
data class AppearanceSummaryLabels(
    val theme: (ThemeMode) -> String,
    val followsSystem: String,
    val textPercent: (Int) -> String,
    val interfacePercent: (Int) -> String,
)

/** Builds the Appearance row subtitle from the saved choices and localized labels. */
fun appearanceRowSummary(
    mode: ThemeMode,
    appearance: Appearance,
    labels: AppearanceSummaryLabels,
): String {
    val textLabel =
        if (appearance.textPercent == TEXT_FOLLOW_SYSTEM) {
            labels.followsSystem
        } else {
            labels.textPercent(appearance.textPercent)
        }
    val parts = mutableListOf(labels.theme(mode), textLabel)
    if (appearance.interfacePercent != FULL_PERCENT) {
        parts += labels.interfacePercent(appearance.interfacePercent)
    }
    return parts.joinToString(separator = " · ")
}

/** The five interface size steps shown by Appearance. */
fun interfaceSizeSteps(): List<Int> = INTERFACE_PERCENTS

/** The six manual text size steps shown by Appearance. */
fun manualTextSizeSteps(): List<Int> = TEXT_PERCENTS.filter { it != TEXT_FOLLOW_SYSTEM }

/** Formats a size step for a segmented control or summary label. */
fun sizeStepLabel(percent: Int): String = "$percent%"

/** A swatch seed without any Compose or Android color types. */
data class AccentChoice(
    val id: String,
    val argb: Int,
)

/** Returns the selected built-in swatch, including a match for a saved custom seed. */
fun selectedAccentId(
    argb: Int?,
    choices: List<AccentChoice>,
): String? = argb?.let { value -> choices.firstOrNull { it.argb == value }?.id }

/** A parsed color or a reason the input is not a color. */
sealed interface HexColorResult {
    data class Valid(
        val argb: Int,
    ) : HexColorResult

    data class Invalid(
        val reason: HexError,
    ) : HexColorResult
}

enum class HexError {
    Empty,
    InvalidLength,
    InvalidCharacters,
}

/** Parses #RGB, #RRGGBB, RRGGBB, and surrounding whitespace into an opaque ARGB color. */
fun parseHexColor(input: String): HexColorResult {
    val value = input.trim().removePrefix("#")
    val invalidError =
        when {
            value.isEmpty() -> HexError.Empty
            value.length != HEX_SHORT_LENGTH && value.length != HEX_LONG_LENGTH ->
                HexError.InvalidLength
            !value.all(::isHexDigit) -> HexError.InvalidCharacters
            else -> null
        }
    val parsedArgb: Int? =
        if (invalidError == null) {
            val expanded =
                if (value.length == HEX_SHORT_LENGTH) {
                    value.map { "$it$it" }.joinToString(separator = "")
                } else {
                    value
                }
            val red = expanded.substring(HEX_RED_START, HEX_GREEN_START).toInt(HEX_RADIX)
            val green = expanded.substring(HEX_GREEN_START, HEX_BLUE_START).toInt(HEX_RADIX)
            val blue = expanded.substring(HEX_BLUE_START, HEX_COLOR_END).toInt(HEX_RADIX)
            argb(red, green, blue)
        } else {
            null
        }
    return parsedArgb?.takeIf { invalidError == null }?.let { HexColorResult.Valid(it) }
        ?: HexColorResult.Invalid(invalidError ?: HexError.InvalidCharacters)
}

/** Formats an opaque ARGB color as the six digit form used by the picker. */
fun formatHexColor(color: Int): String {
    val rgb = color and RGB_MASK
    return "#${rgb.toString(HEX_RADIX).uppercase().padStart(HEX_LONG_LENGTH, '0')}"
}

/** HSV values used by the hue and saturation controls. Value stays fixed while those move. */
data class HsvColor(
    val hue: Float,
    val saturation: Float,
    val value: Float,
)

/** Converts hue, saturation, and value to an opaque ARGB color. */
fun hsvToArgb(
    hue: Float,
    saturation: Float,
    value: Float,
): Int {
    val safeHue = ((hue.toDouble() % HUE_MAX) + HUE_MAX) % HUE_MAX
    val safeSaturation = saturation.coerceIn(HSV_MIN, HSV_MAX).toDouble()
    val safeValue = value.coerceIn(HSV_MIN, HSV_MAX).toDouble()
    val sector = safeHue / HUE_SECTOR
    val sectorIndex = sector.toInt() % HUE_SECTORS
    val fraction = sector - sector.toInt()
    val high = safeValue
    val low = safeValue * (1.0 - safeSaturation)
    val descending = safeValue * (1.0 - safeSaturation * fraction)
    val ascending = safeValue * (1.0 - safeSaturation * (1.0 - fraction))
    val channels =
        when (sectorIndex) {
            RED_SECTOR -> listOf(high, ascending, low)
            YELLOW_SECTOR -> listOf(descending, high, low)
            GREEN_SECTOR -> listOf(low, high, ascending)
            CYAN_SECTOR -> listOf(low, descending, high)
            BLUE_SECTOR -> listOf(ascending, low, high)
            else -> listOf(high, low, descending)
        }
    return argb(
        red = (channels[0] * RGB_MAX).roundToInt(),
        green = (channels[1] * RGB_MAX).roundToInt(),
        blue = (channels[2] * RGB_MAX).roundToInt(),
    )
}

/** Converts an opaque or translucent ARGB color to HSV, ignoring alpha. */
fun argbToHsv(color: Int): HsvColor {
    val red = ((color shr RED_SHIFT) and CHANNEL_MASK).toDouble() / RGB_MAX
    val green = ((color shr GREEN_SHIFT) and CHANNEL_MASK).toDouble() / RGB_MAX
    val blue = (color and CHANNEL_MASK).toDouble() / RGB_MAX
    val maximum = maxOf(red, green, blue)
    val minimum = minOf(red, green, blue)
    val delta = maximum - minimum
    val hue =
        when {
            delta == HSV_MIN.toDouble() -> HSV_MIN.toDouble()
            maximum == red ->
                HUE_SECTOR * (((green - blue) / delta) % HUE_SECTORS.toDouble())
            maximum == green -> HUE_SECTOR * (((blue - red) / delta) + HUE_GREEN_OFFSET)
            else -> HUE_SECTOR * (((red - green) / delta) + HUE_BLUE_OFFSET)
        }
    val normalizedHue = ((hue % HUE_MAX) + HUE_MAX) % HUE_MAX
    val saturation =
        if (maximum == HSV_MIN.toDouble()) HSV_MIN.toDouble() else delta / maximum
    return HsvColor(normalizedHue.toFloat(), saturation.toFloat(), maximum.toFloat())
}

/** True when two RGB colors differ enough for a user to see the readability adjustment. */
fun differsVisibly(
    first: Int,
    second: Int,
): Boolean {
    val redDifference =
        abs(
            ((first shr RED_SHIFT) and CHANNEL_MASK) -
                ((second shr RED_SHIFT) and CHANNEL_MASK),
        )
    val greenDifference =
        abs(
            ((first shr GREEN_SHIFT) and CHANNEL_MASK) -
                ((second shr GREEN_SHIFT) and CHANNEL_MASK),
        )
    val blueDifference = abs((first and CHANNEL_MASK) - (second and CHANNEL_MASK))
    return maxOf(redDifference, greenDifference, blueDifference) > VISIBLE_CHANNEL_DELTA
}

private const val VISIBLE_CHANNEL_DELTA = 4

private fun isHexDigit(character: Char): Boolean =
    character in '0'..'9' || character.lowercaseChar() in 'a'..'f'

private fun argb(
    red: Int,
    green: Int,
    blue: Int,
): Int =
    ALPHA_MASK or
        ((red.coerceIn(0, RGB_MAX)) shl RED_SHIFT) or
        ((green.coerceIn(0, RGB_MAX)) shl GREEN_SHIFT) or
        blue.coerceIn(0, RGB_MAX)
