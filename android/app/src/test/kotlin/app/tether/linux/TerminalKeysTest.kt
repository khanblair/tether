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
}
