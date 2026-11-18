package app.tether.design

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import app.tether.design.tokens.TetherChartColors
import app.tether.design.tokens.TetherColors
import app.tether.design.tokens.TetherFont
import app.tether.design.tokens.TetherPalette
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSyntaxColors
import app.tether.design.tokens.TetherTextToken
import app.tether.design.tokens.TetherTokens
import app.tether.design.tokens.TetherType

/** The theme the user picked. System follows the device setting. */
enum class ThemeMode { System, Light, Dark }

private val SansFamily =
    FontFamily(
        Font(R.font.ibm_plex_sans_regular, FontWeight.Normal),
        Font(R.font.ibm_plex_sans_medium, FontWeight.Medium),
        Font(R.font.ibm_plex_sans_semibold, FontWeight.SemiBold),
    )

private val MonoFamily =
    FontFamily(
        Font(R.font.ibm_plex_mono_regular, FontWeight.Normal),
        Font(R.font.ibm_plex_mono_medium, FontWeight.Medium),
    )

private fun TetherTextToken.toTextStyle(): TextStyle =
    TextStyle(
        fontFamily = if (font == TetherFont.Mono) MonoFamily else SansFamily,
        fontSize = size,
        lineHeight = lineHeight,
        fontWeight = FontWeight(weight),
        letterSpacing = letterSpacing,
    )

private val TetherTypography =
    Typography(
        displayLarge = TetherType.display.toTextStyle(),
        displayMedium = TetherType.display.toTextStyle(),
        displaySmall = TetherType.display.toTextStyle(),
        headlineLarge = TetherType.titleL.toTextStyle(),
        headlineMedium = TetherType.titleL.toTextStyle(),
        headlineSmall = TetherType.titleM.toTextStyle(),
        titleLarge = TetherType.titleL.toTextStyle(),
        titleMedium = TetherType.titleM.toTextStyle(),
        titleSmall = TetherType.titleS.toTextStyle(),
        bodyLarge = TetherType.bodyL.toTextStyle(),
        bodyMedium = TetherType.body.toTextStyle(),
        bodySmall = TetherType.bodyS.toTextStyle(),
        labelLarge = TetherType.label.toTextStyle(),
        labelMedium = TetherType.label.toTextStyle(),
        labelSmall = TetherType.caption.toTextStyle(),
    )

private val TetherShapes =
    Shapes(
        extraSmall = RoundedCornerShape(TetherRadius.xs),
        small = RoundedCornerShape(TetherRadius.sm),
        medium = RoundedCornerShape(TetherRadius.md),
        large = RoundedCornerShape(TetherRadius.lg),
        extraLarge = RoundedCornerShape(TetherRadius.lg),
    )

private fun TetherColors.toMaterial(dark: Boolean): ColorScheme {
    val base = if (dark) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = accent,
        onPrimary = onAccent,
        primaryContainer = accentSoft,
        onPrimaryContainer = accentInk,
        secondary = accent2,
        onSecondary = onAccent2,
        secondaryContainer = tonal,
        onSecondaryContainer = onTonal,
        tertiary = accent2,
        onTertiary = onAccent2,
        background = bg,
        onBackground = text,
        surface = surface,
        onSurface = text,
        surfaceVariant = surfaceSunken,
        onSurfaceVariant = text2,
        surfaceTint = Color.Transparent,
        inverseSurface = text,
        inverseOnSurface = bg,
        inversePrimary = accentInk,
        error = danger,
        onError = dangerSoft,
        errorContainer = dangerSoft,
        onErrorContainer = danger,
        outline = lineInput,
        outlineVariant = line,
        scrim = Color.Black,
        surfaceDim = surfaceSunken,
        surfaceBright = surfaceRaised,
        surfaceContainerLowest = surface,
        surfaceContainerLow = surface,
        surfaceContainer = surface,
        surfaceContainerHigh = surfaceRaised,
        surfaceContainerHighest = surfaceRaised,
    )
}

private val LocalPalette = staticCompositionLocalOf { TetherTokens.light }
private val LocalDark = staticCompositionLocalOf { false }

/** Entry to theme values that Material does not carry: accent ink, diff, syntax, chart. */
object TetherTheme {
    val colors: TetherColors
        @Composable @ReadOnlyComposable
        get() = LocalPalette.current.color
    val syntax: TetherSyntaxColors
        @Composable @ReadOnlyComposable
        get() = LocalPalette.current.syntax
    val chart: TetherChartColors
        @Composable @ReadOnlyComposable
        get() = LocalPalette.current.chart
    val isDark: Boolean
        @Composable @ReadOnlyComposable
        get() = LocalDark.current
    val mono: TextStyle get() = TetherType.mono.toTextStyle()
    val monoSmall: TextStyle get() = TetherType.monoS.toTextStyle()
    val monoLabel: TextStyle get() = TetherType.monoLabel.toTextStyle()
}

/**
 * Applies the Velvet Plum theme. Material dynamic color is off on purpose: Android 12 wallpaper
 * colors would replace the brand palette.
 */
@Composable
fun TetherTheme(
    mode: ThemeMode = ThemeMode.System,
    content: @Composable () -> Unit,
) {
    val dark =
        when (mode) {
            ThemeMode.System -> isSystemInDarkTheme()
            ThemeMode.Light -> false
            ThemeMode.Dark -> true
        }
    val palette: TetherPalette = if (dark) TetherTokens.dark else TetherTokens.light
    CompositionLocalProvider(LocalPalette provides palette, LocalDark provides dark) {
        MaterialTheme(
            colorScheme = palette.color.toMaterial(dark),
            typography = TetherTypography,
            shapes = TetherShapes,
            content = content,
        )
    }
}
