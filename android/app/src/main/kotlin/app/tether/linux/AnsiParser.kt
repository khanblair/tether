package app.tether.linux

/** Byte-fed parser for the small ANSI subset used by interactive Linux programs. */
class AnsiParser(
    val buffer: TerminalBuffer,
) {
    private var state = State.Ground
    private val csi = StringBuilder()
    private var utf8CodePoint = 0
    private var utf8Expected = 0
    private var utf8Minimum = 0
    private var ignoreUntilBel = false

    fun feed(bytes: ByteArray) {
        bytes.forEach { acceptByte(it.toInt() and BYTE_MASK) }
    }

    fun feed(bytes: List<UByte>) {
        feed(ByteArray(bytes.size) { index -> bytes[index].toByte() })
    }

    fun reset() {
        state = State.Ground
        csi.clear()
        utf8CodePoint = 0
        utf8Expected = 0
        utf8Minimum = 0
        ignoreUntilBel = false
    }

    private fun acceptByte(value: Int) {
        when (state) {
            State.Ground -> acceptGround(value)
            State.Escape -> acceptEscape(value)
            State.Csi -> acceptCsi(value)
            State.Ignore -> acceptIgnored(value)
        }
    }

    private fun acceptGround(value: Int) {
        when {
            value == ESC -> beginEscape()
            value == CR -> buffer.carriageReturn()
            value == LF -> buffer.lineFeed()
            value == BACKSPACE -> buffer.backspace()
            value == TAB -> buffer.tab()
            value < FIRST_PRINTABLE -> Unit
            value < ASCII_LIMIT -> buffer.write(value.toChar().toString())
            else -> acceptUtf8(value)
        }
    }

    private fun acceptEscape(value: Int) {
        when (value) {
            '['.code -> {
                state = State.Csi
                csi.clear()
            }
            ']'.code -> {
                state = State.Ignore
                ignoreUntilBel = true
            }
            ESC -> beginEscape()
            else -> state = State.Ground
        }
    }

    private fun acceptCsi(value: Int) {
        when {
            value in FINAL_BYTES -> {
                applyCsi(value.toChar(), csi.toString())
                state = State.Ground
                csi.clear()
            }
            value == ESC -> beginEscape()
            csi.length < MAX_CSI_LENGTH -> csi.append(value.toChar())
            else -> state = State.Ignore
        }
    }

    private fun acceptIgnored(value: Int) {
        if (ignoreUntilBel) {
            if (value == BEL) {
                state = State.Ground
                ignoreUntilBel = false
            } else if (value == ESC) {
                state = State.Escape
            }
        } else if (value == ESC) {
            state = State.Escape
        } else if (value in FINAL_BYTES) {
            state = State.Ground
        }
    }

    private fun beginEscape() {
        state = State.Escape
        utf8Expected = 0
        ignoreUntilBel = false
    }

    private fun acceptUtf8(value: Int) {
        if (utf8Expected == 0) startUtf8(value) else continueUtf8(value)
    }

    private fun startUtf8(value: Int) {
        when {
            value in TWO_BYTE_LEAD ->
                startUtf8Sequence(value and TWO_BYTE_MASK, TWO_BYTE_TRAILING, TWO_BYTE_MIN)
            value in THREE_BYTE_LEAD ->
                startUtf8Sequence(value and THREE_BYTE_MASK, THREE_BYTE_TRAILING, THREE_BYTE_MIN)
            value in FOUR_BYTE_LEAD ->
                startUtf8Sequence(value and FOUR_BYTE_MASK, FOUR_BYTE_TRAILING, FOUR_BYTE_MIN)
            else -> buffer.write("�")
        }
    }

    private fun startUtf8Sequence(
        value: Int,
        expected: Int,
        minimum: Int,
    ) {
        utf8CodePoint = value
        utf8Expected = expected
        utf8Minimum = minimum
    }

    private fun continueUtf8(value: Int) {
        if (value !in CONTINUATION) {
            finishInvalidUtf8(value)
            return
        }
        utf8CodePoint = (utf8CodePoint shl CONTINUATION_BITS) or (value and CONTINUATION_MASK)
        utf8Expected--
        if (utf8Expected == 0) finishUtf8()
    }

    private fun finishUtf8() {
        val value = utf8CodePoint
        if (value < utf8Minimum || value > MAX_CODE_POINT || value in SURROGATES) {
            buffer.write("�")
        } else {
            buffer.write(String(Character.toChars(value)))
        }
        utf8CodePoint = 0
        utf8Minimum = 0
    }

    private fun finishInvalidUtf8(value: Int) {
        buffer.write("�")
        utf8Expected = 0
        utf8CodePoint = 0
        utf8Minimum = 0
        acceptByte(value)
    }

    private enum class State { Ground, Escape, Csi, Ignore }

    private companion object {
        const val ESC = 0x1b
        const val CR = 0x0d
        const val LF = 0x0a
        const val BACKSPACE = 0x08
        const val TAB = 0x09
        const val BEL = 0x07
        const val MAX_CSI_LENGTH = 64

        /** Keeps the low eight bits of a byte read as an Int. */
        const val BYTE_MASK = 0xff

        /** Bytes below this are C0 controls, which print nothing unless handled above. */
        const val FIRST_PRINTABLE = 0x20

        /** Bytes below this are 7-bit ASCII. Bytes at or above it start UTF-8 sequences. */
        const val ASCII_LIMIT = 0x80

        /** Final bytes of CSI sequences and of ignored OSC or escape sequences. */
        val FINAL_BYTES = 0x40..0x7e

        // UTF-8 lead bytes: which range starts each sequence length, the payload bits kept
        // from the lead byte, the count of continuation bytes that follow, and the smallest
        // code point that the sequence length may encode (smaller values are overlong).
        val TWO_BYTE_LEAD = 0xc2..0xdf
        val THREE_BYTE_LEAD = 0xe0..0xef
        val FOUR_BYTE_LEAD = 0xf0..0xf4
        const val TWO_BYTE_MASK = 0x1f
        const val THREE_BYTE_MASK = 0x0f
        const val FOUR_BYTE_MASK = 0x07
        const val TWO_BYTE_TRAILING = 1
        const val THREE_BYTE_TRAILING = 2
        const val FOUR_BYTE_TRAILING = 3
        const val TWO_BYTE_MIN = 0x80
        const val THREE_BYTE_MIN = 0x800
        const val FOUR_BYTE_MIN = 0x10000

        /** Continuation bytes carry six payload bits each. */
        val CONTINUATION = 0x80..0xbf
        const val CONTINUATION_BITS = 6
        const val CONTINUATION_MASK = 0x3f
        const val MAX_CODE_POINT = 0x10ffff
        val SURROGATES = 0xd800..0xdfff
    }
}
