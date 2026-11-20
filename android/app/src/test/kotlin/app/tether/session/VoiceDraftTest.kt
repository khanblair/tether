package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class VoiceDraftTest {
    @Test fun speechFillsAnEmptyDraft() {
        assertEquals("Run the tests", mergeVoiceDraft("", "Run the tests"))
    }

    @Test fun speechDoesNotEraseTypedText() {
        assertEquals("Please run the tests", mergeVoiceDraft("Please", "run the tests"))
    }
}
