package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ComposerTest {
    @Test fun blankTextCannotStartATurn() {
        assertNull(outgoingText(" \n "))
    }

    @Test fun lineBreaksInsideAMessageAreKept() {
        assertEquals("first\nsecond", outgoingText("  first\nsecond  "))
    }
}
