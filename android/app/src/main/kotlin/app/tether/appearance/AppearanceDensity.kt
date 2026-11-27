package app.tether.appearance

import androidx.compose.ui.unit.Density

private const val FULL_PERCENT = 100

/**
 * The density to use for the whole app, given the system [base] density and the saved
 * [appearance].
 *
 * The interface size scales the density. Layout is in dp, so every dp size grows or shrinks.
 * Text is in sp, and pixels = sp x fontScale x density, so the interface size also scales the
 * text. The text size is applied on top of that: it replaces the system font scale, unless it
 * is set to Follow system.
 */
internal fun effectiveDensity(
    base: Density,
    appearance: Appearance,
): Density {
    val fontScale =
        if (appearance.textPercent == TEXT_FOLLOW_SYSTEM) {
            base.fontScale
        } else {
            appearance.textPercent.toFloat() / FULL_PERCENT
        }
    return Density(
        density = base.density * appearance.interfacePercent / FULL_PERCENT,
        fontScale = fontScale,
    )
}
