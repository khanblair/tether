package app.tether.linux

import java.io.ByteArrayOutputStream

/** Keys exposed by the terminal row or the Android keyboard. */
sealed interface TerminalKey {
    data object Escape : TerminalKey

    data object Tab : TerminalKey

    data object Enter : TerminalKey

    data object Backspace : TerminalKey

    data object ArrowUp : TerminalKey

    data object ArrowDown : TerminalKey

    data object ArrowLeft : TerminalKey

    data object ArrowRight : TerminalKey

    data object Ctrl : TerminalKey

    data object Alt : TerminalKey

    data class Character(
        val value: Char,
    ) : TerminalKey
}

/** Stateful mapping for sticky modifiers and application cursor mode. */
class TerminalKeys(
    var applicationCursorMode: Boolean = false,
) {
    var ctrlArmed: Boolean = false
        private set
    var altArmed: Boolean = false
        private set

    fun press(key: TerminalKey): ByteArray =
        when (key) {
            TerminalKey.Ctrl -> {
                ctrlArmed = !ctrlArmed
                ByteArray(0)
            }
            TerminalKey.Alt -> {
                altArmed = !altArmed
                ByteArray(0)
            }
            else -> sendKey(key)
        }

    /**
     * Maps text typed on the Android keyboard through the same sticky modifiers as [press]. The
     * first character takes an armed Ctrl or Alt, and both reset after it.
     */
    fun type(text: String): ByteArray {
        val output = ByteArrayOutputStream()
        var index = 0
        while (index < text.length) {
            val width = Character.charCount(text.codePointAt(index))
            output.write(
                if (width == 1) {
                    press(TerminalKey.Character(text[index]))
                } else {
                    typeLiteral(text.substring(index, index + width))
                },
            )
            index += width
        }
        return output.toByteArray()
    }

    private fun typeLiteral(text: String): ByteArray {
        val alt = altArmed
        ctrlArmed = false
        altArmed = false
        val bytes = text.toByteArray(Charsets.UTF_8)
        return if (alt) byteArrayOf(ESC) + bytes else bytes
    }

    private fun sendKey(key: TerminalKey): ByteArray {
        val alt = altArmed
        val bytes = mapKey(key)
        ctrlArmed = false
        altArmed = false
        return if (alt) byteArrayOf(ESC) + bytes else bytes
    }

    private fun mapKey(key: TerminalKey): ByteArray =
        when (key) {
            TerminalKey.Escape -> byteArrayOf(ESC)
            TerminalKey.Tab -> byteArrayOf(TAB)
            TerminalKey.Enter -> byteArrayOf(CR)
            TerminalKey.Backspace -> byteArrayOf(DEL)
            TerminalKey.ArrowUp -> arrow('A')
            TerminalKey.ArrowDown -> arrow('B')
            TerminalKey.ArrowRight -> arrow('C')
            TerminalKey.ArrowLeft -> arrow('D')
            is TerminalKey.Character -> character(key.value)
            TerminalKey.Ctrl, TerminalKey.Alt -> ByteArray(0)
        }

    private fun character(value: Char): ByteArray {
        val lower = value.lowercaseChar()
        return if (ctrlArmed && lower in 'a'..'z') {
            byteArrayOf((lower.code - 'a'.code + 1).toByte())
        } else {
            value.toString().toByteArray(Charsets.UTF_8)
        }
    }

    private fun arrow(final: Char): ByteArray =
        if (applicationCursorMode) {
            byteArrayOf(ESC, 'O'.code.toByte(), final.code.toByte())
        } else {
            byteArrayOf(ESC, '['.code.toByte(), final.code.toByte())
        }

    private companion object {
        const val ESC: Byte = 0x1b
        const val TAB: Byte = 0x09
        const val CR: Byte = 0x0d
        const val DEL: Byte = 0x7f
    }
}
