package app.tether.session.tools

import app.tether.session.agent.ToolStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ToolStatusPresentationTest {
    @Test
    fun every_status_has_a_presentation_with_a_glyph_and_a_label() {
        ToolStatus.entries.forEach { status ->
            val presentation = toolStatusPresentation(status)
            assertTrue("no glyph for $status", ToolStatusGlyph.entries.contains(presentation.glyph))
            assertTrue("no label for $status", ToolStatusLabel.entries.contains(presentation.label))
        }
    }

    @Test
    fun every_status_label_is_used_by_exactly_one_status() {
        val labels = ToolStatus.entries.map { toolStatusPresentation(it).label }
        assertEquals(ToolStatusLabel.entries.size, labels.toSet().size)
        assertEquals(ToolStatusLabel.entries.toSet(), labels.toSet())
    }

    @Test
    fun only_failed_timed_out_and_blocked_use_the_danger_tone() {
        val danger =
            ToolStatus.entries.filter { toolStatusPresentation(it).tone == ToolTone.Danger }.toSet()
        assertEquals(setOf(ToolStatus.Failed, ToolStatus.TimedOut, ToolStatus.Blocked), danger)
    }

    @Test
    fun succeeded_is_the_only_success_tone() {
        val success =
            ToolStatus.entries.filter { toolStatusPresentation(it).tone == ToolTone.Success }
        assertEquals(listOf(ToolStatus.Succeeded), success)
    }

    @Test
    fun awaiting_approval_shows_a_pause_and_needs_approval() {
        val presentation = toolStatusPresentation(ToolStatus.AwaitingApproval)
        assertEquals(ToolStatusGlyph.Pause, presentation.glyph)
        assertEquals(ToolStatusLabel.NeedsApproval, presentation.label)
        assertFalse(presentation.tone == ToolTone.Danger)
    }

    @Test
    fun denied_is_neutral_because_the_user_chose_it() {
        assertEquals(ToolTone.Neutral, toolStatusPresentation(ToolStatus.Denied).tone)
    }
}
