package app.tether.session

import app.tether.design.components.ChipTone
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ComposerRulesTest {
    @Test
    fun autoIsAccentBypassIsDangerAndOtherModesAreNeutral() {
        assertEquals(ChipTone.Accent, modeChipTone(SessionMode.AUTO))
        assertEquals(ChipTone.Danger, modeChipTone(SessionMode.BYPASS))
        assertEquals(ChipTone.Neutral, modeChipTone(SessionMode.PLAN))
        assertEquals(ChipTone.Neutral, modeChipTone(SessionMode.ASK))
        assertEquals(ChipTone.Neutral, modeChipTone(SessionMode.ACCEPT_EDITS))
    }

    @Test
    fun photoNeedsAVisionModel() {
        assertEquals(PlusBlock.NO_VISION, plusRowBlock(PlusRow.PHOTO, false, ""))
        assertNull(plusRowBlock(PlusRow.PHOTO, true, ""))
    }

    @Test
    fun cameraNeedsAVisionModelLikePhoto() {
        assertEquals(PlusBlock.NO_VISION, plusRowBlock(PlusRow.CAMERA, false, ""))
        assertNull(plusRowBlock(PlusRow.CAMERA, true, ""))
    }

    @Test
    fun cameraFileNameUsesTheCaptureTime() {
        assertEquals("photo-1234.jpg", cameraFileName(1234L))
    }

    @Test
    fun fileIsAlwaysOffUntilTheBackendExists() {
        assertEquals(PlusBlock.NOT_AVAILABLE, plusRowBlock(PlusRow.FILE, true, ""))
    }

    @Test
    fun mentionIsAlwaysOn() {
        assertNull(plusRowBlock(PlusRow.MENTION, false, "some draft"))
    }

    @Test
    fun commandsNeedAnEmptyDraft() {
        assertNull(plusRowBlock(PlusRow.COMMANDS, false, ""))
        assertEquals(PlusBlock.DRAFT_NOT_EMPTY, plusRowBlock(PlusRow.COMMANDS, false, "fix"))
    }

    @Test
    fun mentionAtTheEndOfAWordAddsASpaceFirst() {
        assertEquals(DraftEdit("fix @", 5), mentionEdit("fix", 3, 3))
    }

    @Test
    fun mentionAfterASpaceOrInEmptyDraftNeedsNoSpace() {
        assertEquals(DraftEdit("fix @", 5), mentionEdit("fix ", 4, 4))
        assertEquals(DraftEdit("@", 1), mentionEdit("", 0, 0))
    }

    @Test
    fun mentionReplacesTheSelection() {
        assertEquals(DraftEdit("see @", 5), mentionEdit("see a.kt", 4, 8))
        assertEquals(DraftEdit("x @ z", 3), mentionEdit("x y z", 2, 3))
    }

    @Test
    fun commandGoesInOnlyIntoAnEmptyDraft() {
        assertEquals(DraftEdit("/", 1), commandEdit(""))
        assertNull(commandEdit("fix"))
    }
}
