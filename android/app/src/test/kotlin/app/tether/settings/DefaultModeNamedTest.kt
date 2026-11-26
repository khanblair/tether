package app.tether.settings

import app.tether.session.SessionMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DefaultModeNamedTest {
    @Test
    fun no_saved_name_means_no_default() {
        assertNull(defaultModeNamed(null))
    }

    @Test
    fun a_saved_bypass_is_never_a_default() {
        assertNull(defaultModeNamed(SessionMode.BYPASS.name))
    }

    @Test
    fun a_saved_plain_mode_is_the_default() {
        assertEquals(SessionMode.ASK, defaultModeNamed(SessionMode.ASK.name))
    }

    @Test
    fun an_unknown_name_is_no_default() {
        assertNull(defaultModeNamed("NOT_A_MODE"))
    }
}
