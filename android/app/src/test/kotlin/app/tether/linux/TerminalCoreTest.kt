package app.tether.linux

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TerminalCoreTest {
    @Test
    fun parserTableCoversTheSupportedTerminalSubset() {
        val cases = parserCases()
        assertTrue(cases.size >= 40)
        cases.forEach { case ->
            val buffer = TerminalBuffer(rows = 4, columns = 4)
            case.action(AnsiParser(buffer))
            assertTrue(case.name, buffer.lines().isNotEmpty())
        }
    }

    @Test
    fun parserAppliesAttributesAndAlternateScreen() {
        val buffer = TerminalBuffer(rows = 3, columns = 8)
        val parser = AnsiParser(buffer)
        parser.feed("main")
        parser.feed("\u001b[31;1mred\u001b[?1049halt\u001b[?1049l")
        assertEquals("mainred", buffer.lastNonBlankLine().trim())
        assertNull(
            buffer
                .lines()
                .first()[0]
                .attribute.foreground,
        )
        assertEquals(
            TerminalColor(1),
            buffer
                .lines()
                .first()[4]
                .attribute.foreground,
        )
        assertTrue(
            buffer
                .lines()
                .first()[4]
                .attribute.bold,
        )
        assertFalse(buffer.bracketedPaste)
    }

    @Test
    fun parserDecodesSplitUtf8AndEmoji() {
        val buffer = TerminalBuffer(rows = 2, columns = 12)
        val parser = AnsiParser(buffer)
        parser.feed(byteArrayOf(0xc3.toByte()))
        assertEquals(" ", text(buffer.lines().last()).first().toString())
        parser.feed(byteArrayOf(0xa9.toByte()))
        parser.feed("🙂".toByteArray())
        assertEquals("é🙂", buffer.lastNonBlankLine().take(3))
    }

    @Test
    fun parserKeepsIncompleteAndUnknownSequencesOutOfText() {
        val buffer = TerminalBuffer(rows = 2, columns = 12)
        val parser = AnsiParser(buffer)
        parser.feed("a\u001b[31".toByteArray())
        assertEquals("a", buffer.lastNonBlankLine().take(1))
        parser.feed("mq\u001b[99rZ".toByteArray())
        assertEquals("aqZ", buffer.lastNonBlankLine().take(3))
        assertEquals(
            TerminalColor(1),
            buffer
                .lines()
                .first()[1]
                .attribute.foreground,
        )
    }

    @Test
    fun tabStopsAreEveryEightColumns() {
        val buffer = TerminalBuffer(rows = 2, columns = 20)
        AnsiParser(buffer).feed("a\tb")
        assertEquals(9, buffer.cursorColumn)
        assertEquals("a       b", buffer.lastNonBlankLine().take(9))
    }

    @Test
    fun bufferScrollbackIsBounded() {
        val buffer = TerminalBuffer(rows = 2, columns = 4)
        repeat(TerminalBuffer.MAX_SCROLLBACK + 20) { buffer.lineFeed() }
        assertEquals(TerminalBuffer.MAX_SCROLLBACK + 2, buffer.lines().size)
    }

    @Test
    fun resizeTruncatesAndPadsWithoutReflow() {
        val buffer = TerminalBuffer(rows = 2, columns = 4)
        buffer.write("abcd")
        buffer.setCursor(1, 0)
        buffer.write("xy")
        buffer.resize(3, 3)
        assertEquals("abc", text(buffer.lines()[0]))
        assertEquals(
            "xy ",
            text(buffer.lines()[1]),
        )
        assertEquals("   ", text(buffer.lines()[2]))
    }

    private data class ParserCase(
        val name: String,
        val action: AnsiParser.() -> Unit,
    )

    private fun parserCases(): List<ParserCase> = textAndCursorCases() + styleAndModeCases()

    private fun textAndCursorCases(): List<ParserCase> =
        listOf(
            ParserCase("plain text") { feed("hello") },
            ParserCase("spaces") { feed("a b") },
            ParserCase("punctuation") { feed("$#_!") },
            ParserCase("right edge wraps") { feed("1234") },
            ParserCase("second wrapped line") { feed("5") },
            ParserCase("scrolls into history") { feed("\n\n\n\n") },
            ParserCase("carriage return") { feed("abc\rZ") },
            ParserCase("line feed") { feed("a\nb") },
            ParserCase("carriage return and line feed") { feed("a\r\nb") },
            ParserCase("backspace") { feed("ab\bZ") },
            ParserCase("tab stop") { feed("a\tb") },
            ParserCase("cursor up") { feed("a\u001b[AZ") },
            ParserCase("cursor down") { feed("\u001b[BZ") },
            ParserCase("cursor forward") { feed("a\u001b[2CZ") },
            ParserCase("cursor back") { feed("abc\u001b[2DZ") },
            ParserCase("cursor H") { feed("\u001b[2;3HX") },
            ParserCase("cursor f") { feed("\u001b[1;2fX") },
            ParserCase("cursor clamps high") { feed("\u001b[99;99HX") },
            ParserCase("cursor clamps low") { feed("\u001b[1;1H\u001b[99DX") },
            ParserCase("erase line to end") { feed("abc\u001b[2GX\u001b[0K") },
            ParserCase("erase line to start") { feed("abc\u001b[1K") },
            ParserCase("erase whole line") { feed("abc\u001b[2K") },
            ParserCase("erase display to end") { feed("abc\u001b[0J") },
            ParserCase("erase display to start") { feed("abc\u001b[1J") },
            ParserCase("erase whole display") { feed("abc\u001b[2J") },
        )

    private fun styleAndModeCases(): List<ParserCase> =
        listOf(
            ParserCase("basic foreground") { feed("\u001b[31mR") },
            ParserCase("basic background") { feed("\u001b[44mB") },
            ParserCase("bright foreground") { feed("\u001b[91mR") },
            ParserCase("bright background") { feed("\u001b[103mB") },
            ParserCase("bold") { feed("\u001b[1mB") },
            ParserCase("underline") { feed("\u001b[4mU") },
            ParserCase("inverse") { feed("\u001b[7mI") },
            ParserCase("reset") { feed("\u001b[1;4;7mX\u001b[0mY") },
            ParserCase("foreground 256") { feed("\u001b[38;5;200mX") },
            ParserCase("background 256") { feed("\u001b[48;5;24mX") },
            ParserCase("foreground truecolor") { feed("\u001b[38;2;1;2;3mX") },
            ParserCase("background truecolor") { feed("\u001b[48;2;3;2;1mX") },
            ParserCase("utf8 split") {
                feed(byteArrayOf(0xc3.toByte()))
                feed(byteArrayOf(0xa9.toByte()))
            },
            ParserCase("emoji") { feed("🙂") },
            ParserCase("unknown csi") { feed("a\u001b[99qZ") },
            ParserCase("incomplete sequence") {
                feed("a\u001b[31")
                feed("mZ")
            },
            ParserCase("alternate screen") { feed("main\u001b[?1049halt\u001b[?1049l") },
            ParserCase("bracketed paste") { feed("\u001b[?2004h") },
        )

    private fun text(line: TerminalLine): String =
        line.joinToString(separator = "") { it.character }

    /** The text of the last line that has any content, with trailing blanks removed. */
    private fun TerminalBuffer.lastNonBlankLine(): String =
        lines().map { text(it).trimEnd() }.lastOrNull { it.isNotEmpty() }.orEmpty()
}

private fun AnsiParser.feed(text: String) = feed(text.toByteArray())
