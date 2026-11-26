package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class ExportMarkdownTest {
    @Test
    fun the_words_of_the_title_are_joined_by_dashes() {
        assertEquals("Fix-login-bug.md", exportFileName("Fix login bug"))
    }

    @Test
    fun other_characters_are_dropped() {
        assertEquals("ab-cd.md", exportFileName("a/b: c?d"))
    }

    @Test
    fun a_blank_or_empty_title_falls_back() {
        assertEquals("tether-session.md", exportFileName(""))
        assertEquals("tether-session.md", exportFileName("   "))
        assertEquals("tether-session.md", exportFileName("***"))
    }

    @Test
    fun a_long_title_is_cut_to_sixty_characters() {
        val name = exportFileName("a".repeat(80))
        assertEquals("a".repeat(60) + ".md", name)
    }
}
