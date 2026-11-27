package app.tether.linux

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
