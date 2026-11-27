package app.tether.linux

internal fun AnsiParser.applyCsi(
    final: Char,
    raw: String,
) {
    val privateMode = raw.startsWith("?")
    val params = parseCsiParams(raw.removePrefix("?"))
    when {
        privateMode && final == 'h' -> applyPrivateMode(params, true)
        privateMode && final == 'l' -> applyPrivateMode(params, false)
        final == 'A' -> buffer.moveCursor(-param(params, 0, 1), 0)
        final == 'B' -> buffer.moveCursor(param(params, 0, 1), 0)
        final == 'C' -> buffer.moveCursor(0, param(params, 0, 1))
        final == 'D' -> buffer.moveCursor(0, -param(params, 0, 1))
        final == 'H' || final == 'f' ->
            buffer.setCursor(
                param(params, 0, 1) - 1,
                param(params, 1, 1) - 1,
            )
        final == 'J' -> buffer.eraseDisplay(param(params, 0, 0))
        final == 'K' -> buffer.eraseLine(param(params, 0, 0))
        final == 'm' -> applySgr(params)
    }
}

private fun AnsiParser.applyPrivateMode(
    params: List<Int>,
    enabled: Boolean,
) {
    params.forEach { mode ->
        when (mode) {
            PrivateMode.ALTERNATE_SCREEN -> buffer.useAlternate(enabled)
            PrivateMode.BRACKETED_PASTE -> buffer.setBracketedPaste(enabled)
        }
    }
}

private fun parseCsiParams(raw: String): List<Int> =
    if (raw.isEmpty()) emptyList() else raw.split(';').map { it.toIntOrNull() ?: 0 }

private fun param(
    params: List<Int>,
    index: Int,
    default: Int,
): Int = params.getOrNull(index)?.takeIf { it > 0 } ?: default

private fun AnsiParser.applySgr(params: List<Int>) {
    if (params.isEmpty()) {
        buffer.setAttribute(TerminalAttribute())
        return
    }
    var attribute = buffer.currentAttribute
    var index = 0
    while (index < params.size) {
        val code = params[index]
        if (code == Sgr.EXTENDED_FOREGROUND || code == Sgr.EXTENDED_BACKGROUND) {
            val color = extendedColor(params, index)
            if (color != null) {
                attribute = attribute.withExtendedColor(code == Sgr.EXTENDED_FOREGROUND, color)
                index += colorParamCount(params, index)
            }
        } else {
            attribute = attribute.applySgrCode(code)
        }
        index++
    }
    buffer.setAttribute(attribute)
}

private fun TerminalAttribute.withExtendedColor(
    foreground: Boolean,
    color: TerminalColor,
): TerminalAttribute = if (foreground) copy(foreground = color) else copy(background = color)

private fun TerminalAttribute.applySgrCode(code: Int): TerminalAttribute =
    applyStyleCode(code) ?: applyColorCode(code)

/** Handles the reset, bold, underline and inverse codes. Returns null for any other code. */
private fun TerminalAttribute.applyStyleCode(code: Int): TerminalAttribute? =
    when (code) {
        Sgr.RESET -> TerminalAttribute()
        Sgr.BOLD -> copy(bold = true)
        Sgr.BOLD_OFF -> copy(bold = false)
        Sgr.UNDERLINE -> copy(underline = true)
        Sgr.UNDERLINE_OFF -> copy(underline = false)
        Sgr.INVERSE -> copy(inverse = true)
        Sgr.INVERSE_OFF -> copy(inverse = false)
        else -> null
    }

/** Handles the palette colour codes and the default-colour codes. Unknown codes change nothing. */
private fun TerminalAttribute.applyColorCode(code: Int): TerminalAttribute =
    when (code) {
        Sgr.DEFAULT_FOREGROUND -> copy(foreground = null)
        Sgr.DEFAULT_BACKGROUND -> copy(background = null)
        in Sgr.FOREGROUND -> copy(foreground = TerminalColor(code - Sgr.FOREGROUND.first))
        in Sgr.BACKGROUND -> copy(background = TerminalColor(code - Sgr.BACKGROUND.first))
        in Sgr.BRIGHT_FOREGROUND ->
            copy(foreground = TerminalColor(code - Sgr.BRIGHT_FOREGROUND.first + Sgr.BRIGHT_OFFSET))
        in Sgr.BRIGHT_BACKGROUND ->
            copy(background = TerminalColor(code - Sgr.BRIGHT_BACKGROUND.first + Sgr.BRIGHT_OFFSET))
        else -> this
    }

private fun extendedColor(
    params: List<Int>,
    index: Int,
): TerminalColor? =
    when (params.getOrNull(index + 1)) {
        Sgr.INDEXED_MODE -> params.getOrNull(index + Sgr.EXTENDED_VALUE_OFFSET)?.let(::paletteColor)
        Sgr.RGB_MODE -> rgbColor(params, index)
        else -> null
    }

private fun paletteColor(index: Int): TerminalColor =
    TerminalColor(index.coerceIn(0, Sgr.MAX_CHANNEL))

private fun rgbColor(
    params: List<Int>,
    index: Int,
): TerminalColor? {
    val first = index + Sgr.EXTENDED_VALUE_OFFSET
    val red = params.getOrNull(first)
    val green = params.getOrNull(first + 1)
    val blue = params.getOrNull(first + 2)
    if (red == null || green == null || blue == null) return null
    val packed =
        (red.coerceIn(0, Sgr.MAX_CHANNEL) shl Sgr.RED_SHIFT) or
            (green.coerceIn(0, Sgr.MAX_CHANNEL) shl Sgr.GREEN_SHIFT) or
            blue.coerceIn(0, Sgr.MAX_CHANNEL)
    return TerminalColor(packed, true)
}

private fun colorParamCount(
    params: List<Int>,
    index: Int,
): Int =
    when (params.getOrNull(index + 1)) {
        Sgr.INDEXED_MODE -> Sgr.INDEXED_PARAM_COUNT
        Sgr.RGB_MODE -> Sgr.RGB_PARAM_COUNT
        else -> 0
    }

/** DEC private modes set with CSI ? Pm h and reset with CSI ? Pm l. */
private object PrivateMode {
    const val ALTERNATE_SCREEN = 1049
    const val BRACKETED_PASTE = 2004
}

/** SGR (select graphic rendition) codes and the extended colour forms. */
private object Sgr {
    const val RESET = 0
    const val BOLD = 1
    const val BOLD_OFF = 22
    const val UNDERLINE = 4
    const val UNDERLINE_OFF = 24
    const val INVERSE = 7
    const val INVERSE_OFF = 27
    const val DEFAULT_FOREGROUND = 39
    const val DEFAULT_BACKGROUND = 49
    const val EXTENDED_FOREGROUND = 38
    const val EXTENDED_BACKGROUND = 48

    val FOREGROUND = 30..37
    val BACKGROUND = 40..47
    val BRIGHT_FOREGROUND = 90..97
    val BRIGHT_BACKGROUND = 100..107

    /** Added to a basic colour index to get the bright palette entry. */
    const val BRIGHT_OFFSET = 8

    /** After 38 or 48, the value that selects a 256-colour palette index. */
    const val INDEXED_MODE = 5

    /** After 38 or 48, the value that selects a packed RGB colour. */
    const val RGB_MODE = 2

    /** The number of parameters after 38 or 48 that a palette index consumes. */
    const val INDEXED_PARAM_COUNT = 2

    /** The number of parameters after 38 or 48 that an RGB colour consumes. */
    const val RGB_PARAM_COUNT = 4

    /** Offset from the 38 or 48 code to the first value in an extended colour. */
    const val EXTENDED_VALUE_OFFSET = 2

    const val MAX_CHANNEL = 255
    const val RED_SHIFT = 16
    const val GREEN_SHIFT = 8
}
