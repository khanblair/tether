package app.tether.appearance

/** Text size that follows the system font scale. Stored as 0. */
const val TEXT_FOLLOW_SYSTEM = 0

/** Interface sizes the user can pick, in percent. */
val INTERFACE_PERCENTS = listOf(80, 90, 100, 110, 125)

/** Text sizes the user can pick, in percent. [TEXT_FOLLOW_SYSTEM] is the first entry. */
val TEXT_PERCENTS = listOf(TEXT_FOLLOW_SYSTEM, 80, 90, 100, 115, 130, 150)

private const val DEFAULT_INTERFACE_PERCENT = 100

/**
 * The saved appearance choices.
 *
 * [accentArgb] is null when the user has not picked an accent. Call [normalized] before use,
 * so that values from an older or corrupt store snap to the allowed choices.
 */
data class Appearance(
    val interfacePercent: Int = DEFAULT_INTERFACE_PERCENT,
    val textPercent: Int = TEXT_FOLLOW_SYSTEM,
    val accentArgb: Int? = null,
) {
    /**
     * Returns a copy with allowed values only. An interface size snaps to the nearest allowed
     * size. An unknown text size becomes [TEXT_FOLLOW_SYSTEM]. An accent of 0 becomes null.
     */
    fun normalized(): Appearance =
        Appearance(
            interfacePercent = nearestInterfacePercent(interfacePercent),
            textPercent = if (textPercent in TEXT_PERCENTS) textPercent else TEXT_FOLLOW_SYSTEM,
            accentArgb = accentArgb?.takeIf { it != 0 },
        )
}

private fun nearestInterfacePercent(percent: Int): Int =
    INTERFACE_PERCENTS.minBy { kotlin.math.abs(it - percent) }
