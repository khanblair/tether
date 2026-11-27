package app.tether.linux

import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import app.tether.design.TetherTheme

/** Palette indexes from the ANSI 16-colour table, and the constants of the 256-colour table. */
private object AnsiColor {
    const val RED = 1
    const val GREEN = 2
    const val YELLOW = 3
    const val BLUE = 4
    const val MAGENTA = 5
    const val CYAN = 6
    const val WHITE = 7
    const val GREY = 8
    const val BRIGHT_RED = 9
    const val BRIGHT_WHITE = 15

    /** The bright colours are the normal ones plus this offset. */
    const val BRIGHT_OFFSET = 8

    /** The 6x6x6 colour cube starts here and runs to [LAST_CUBE]. */
    const val FIRST_CUBE = 16
    const val LAST_CUBE = 255

    /** The grey ramp starts here and runs to [LAST_CUBE]. */
    const val GREY_RAMP_START = 232

    /** Each side of the colour cube has this many levels. */
    const val CUBE_SIDE = 6
    const val CUBE_LAYER = 36
    const val CUBE_BASE = 55
    const val CUBE_STEP = 40
    const val GREY_BASE = 8
    const val GREY_STEP = 10

    /** A colour channel is a byte. Channels are divided by this to get a Compose float. */
    const val CHANNEL_MAX = 255f
    const val BYTE_MASK = 0xff
    const val RED_SHIFT = 16
    const val GREEN_SHIFT = 8
}

/** Maps a cell's attributes to the span style its text is drawn with. */
@Composable
internal fun TerminalCell.style(cursor: Boolean): SpanStyle {
    val foreground = colorOf(attribute.foreground, foreground = true)
    val background = colorOf(attribute.background, foreground = false)
    return SpanStyle(
        color = if (attribute.inverse) background else foreground,
        background =
            if (cursor) {
                TetherTheme.colors.accentSoft
            } else if (attribute.inverse) {
                foreground
            } else {
                background
            },
        fontWeight = if (attribute.bold) FontWeight.Bold else FontWeight.Normal,
        textDecoration = if (attribute.underline) TextDecoration.Underline else TextDecoration.None,
    )
}

@Composable
private fun colorOf(
    color: TerminalColor?,
    foreground: Boolean,
): Color =
    when {
        color == null -> defaultColor(foreground)
        color.trueColor -> rgbColor(color.value)
        color.value in AnsiColor.FIRST_CUBE..AnsiColor.LAST_CUBE -> color256(color.value)
        color.value in AnsiColor.RED..AnsiColor.BRIGHT_WHITE -> basicColor(color.value, foreground)
        else -> defaultColor(foreground)
    }

/** The colour used when a cell names no colour. */
@Composable
private fun defaultColor(foreground: Boolean): Color =
    if (foreground) TetherTheme.colors.text else TetherTheme.colors.codeBg

private fun rgbColor(value: Int): Color =
    Color(
        red = ((value shr AnsiColor.RED_SHIFT) and AnsiColor.BYTE_MASK) / AnsiColor.CHANNEL_MAX,
        green = ((value shr AnsiColor.GREEN_SHIFT) and AnsiColor.BYTE_MASK) / AnsiColor.CHANNEL_MAX,
        blue = (value and AnsiColor.BYTE_MASK) / AnsiColor.CHANNEL_MAX,
    )

/** The 16 ANSI colours. Bright colours reuse the theme accents of their normal twin. */
@Composable
private fun basicColor(
    index: Int,
    foreground: Boolean,
): Color {
    val colors = TetherTheme.colors
    val slot = if (index >= AnsiColor.BRIGHT_RED) index - AnsiColor.BRIGHT_OFFSET else index
    return when {
        index == AnsiColor.BRIGHT_WHITE -> colors.surface
        index == AnsiColor.GREY -> colors.text3
        index == AnsiColor.WHITE -> if (foreground) colors.text else colors.surface
        slot in AnsiColor.RED..AnsiColor.CYAN -> accentOf(slot)
        else -> defaultColor(foreground)
    }
}

/** The theme accent for a normal colour slot from red (1) to cyan (6). */
@Composable
private fun accentOf(slot: Int): Color {
    val colors = TetherTheme.colors
    val accents =
        listOf(
            colors.danger,
            colors.success,
            colors.warning,
            colors.info,
            colors.accent2,
            colors.accent,
        )
    return accents[slot - AnsiColor.RED]
}

private fun color256(value: Int): Color =
    if (value < AnsiColor.GREY_RAMP_START) {
        cubeColor(value - AnsiColor.FIRST_CUBE)
    } else {
        greyColor(value)
    }

/** A colour from the 6x6x6 cube. The index runs from 0 to 215. */
private fun cubeColor(index: Int): Color {
    val side = AnsiColor.CUBE_SIDE
    val red = AnsiColor.CUBE_BASE + AnsiColor.CUBE_STEP * (index / AnsiColor.CUBE_LAYER)
    val green = AnsiColor.CUBE_BASE + AnsiColor.CUBE_STEP * ((index / side) % side)
    val blue = AnsiColor.CUBE_BASE + AnsiColor.CUBE_STEP * (index % side)
    return Color(
        red / AnsiColor.CHANNEL_MAX,
        green / AnsiColor.CHANNEL_MAX,
        blue / AnsiColor.CHANNEL_MAX,
    )
}

/** A grey from the ramp at 232 to 255. */
private fun greyColor(value: Int): Color {
    val level =
        (AnsiColor.GREY_BASE + AnsiColor.GREY_STEP * (value - AnsiColor.GREY_RAMP_START)) /
            AnsiColor.CHANNEL_MAX
    return Color(level, level, level)
}
