package app.tether.linux

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TerminalKeysTest {
    @Test
    fun mapsControlLetters() {
        val keys = TerminalKeys()
        keys.press(TerminalKey.Ctrl)
        assertArrayEquals(byteArrayOf(1), keys.press(TerminalKey.Character('A')))
        assertFalse(keys.ctrlArmed)
    }

    @Test
    fun mapsArrowsInNormalAndApplicationModes() {
        val keys = TerminalKeys()
        assertArrayEquals(
            byteArrayOf(0x1b, '['.code.toByte(), 'A'.code.toByte()),
            keys.press(TerminalKey.ArrowUp),
        )
        keys.applicationCursorMode = true
        assertArrayEquals(
            byteArrayOf(0x1b, 'O'.code.toByte(), 'D'.code.toByte()),
            keys.press(TerminalKey.ArrowLeft),
        )
    }

    @Test
    fun mapsAltAndSpecialKeys() {
        val keys = TerminalKeys()
        keys.press(TerminalKey.Alt)
        assertArrayEquals(
            byteArrayOf(0x1b, 'x'.code.toByte()),
            keys.press(TerminalKey.Character('x')),
        )
        assertArrayEquals(byteArrayOf(0x1b), keys.press(TerminalKey.Escape))
        assertArrayEquals(byteArrayOf(0x09), keys.press(TerminalKey.Tab))
        assertArrayEquals(byteArrayOf(0x0d), keys.press(TerminalKey.Enter))
        assertArrayEquals(byteArrayOf(0x7f), keys.press(TerminalKey.Backspace))
    }

    @Test
    fun stickyModifiersCanBeToggledOff() {
        val keys = TerminalKeys()
        keys.press(TerminalKey.Ctrl)
        keys.press(TerminalKey.Ctrl)
        assertFalse(keys.ctrlArmed)
        keys.press(TerminalKey.Alt)
        keys.press(TerminalKey.Alt)
        assertFalse(keys.altArmed)
        assertTrue(keys.press(TerminalKey.Character('|')).contentEquals("|".toByteArray()))
    }

    @Test
    fun keyboardTextTakesTheArmedControlOnce() {
        val keys = TerminalKeys()
        keys.press(TerminalKey.Ctrl)
        assertArrayEquals(byteArrayOf(3, 'b'.code.toByte()), keys.type("cb"))
        assertFalse(keys.ctrlArmed)
        assertArrayEquals("cb".toByteArray(), keys.type("cb"))
    }

    @Test
    fun keyboardTextTakesTheArmedAltOnce() {
        val keys = TerminalKeys()
        keys.press(TerminalKey.Alt)
        assertArrayEquals(byteArrayOf(0x1b, 'x'.code.toByte(), 'y'.code.toByte()), keys.type("xy"))
        assertFalse(keys.altArmed)
    }

    @Test
    fun keyboardTextKeepsNonBmpCharactersWhole() {
        val keys = TerminalKeys()
        val text = "a\u00e9\uD83D\uDE00"
        assertArrayEquals(text.toByteArray(Charsets.UTF_8), keys.type(text))
        keys.press(TerminalKey.Alt)
        keys.press(TerminalKey.Ctrl)
        assertArrayEquals(
            byteArrayOf(0x1b) + "\uD83D\uDE00".toByteArray(Charsets.UTF_8),
            keys.type("\uD83D\uDE00"),
        )
        assertFalse(keys.altArmed)
        assertFalse(keys.ctrlArmed)
    }

    @Test
    fun keyboardSharesModifiersBetweenKeyRowAndTypedText() {
        val sent = mutableListOf<ByteArray>()
        val keyboard = TerminalKeyboard { sent += it }

        keyboard.press(TerminalKey.Ctrl)
        assertTrue(keyboard.ctrlArmed)
        assertTrue(sent.isEmpty())

        keyboard.type("c")
        assertFalse(keyboard.ctrlArmed)
        assertArrayEquals(byteArrayOf(3), sent.single())
    }
}
