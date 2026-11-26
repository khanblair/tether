package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class SessionTitleTest {
    @Test
    fun a_blank_title_shows_the_untitled_label() {
        assertEquals("New session", sessionTitleOr("", "New session"))
        assertEquals("New session", sessionTitleOr("   ", "New session"))
    }

    @Test
    fun a_real_title_is_shown_as_it_is() {
        assertEquals("Fix login", sessionTitleOr("Fix login", "New session"))
    }
}
