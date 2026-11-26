package app.tether.settings

import app.tether.session.ThinkingEffort
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DefaultEffortNamedTest {
    @Test
    fun a_value_never_saved_reads_as_medium() {
        assertEquals(ThinkingEffort.MEDIUM, defaultEffortNamed(null))
    }

    @Test
    fun the_stored_none_is_the_model_default() {
        assertNull(defaultEffortNamed("none"))
    }

    @Test
    fun an_unknown_name_reads_as_medium() {
        assertEquals(ThinkingEffort.MEDIUM, defaultEffortNamed("turbo"))
    }

    @Test
    fun each_level_is_saved_in_lowercase_and_reads_back() {
        assertEquals("low", ThinkingEffort.LOW.storedText())
        ThinkingEffort.entries.forEach { level ->
            assertEquals(level, defaultEffortNamed(level.storedText()))
        }
    }

    @Test
    fun the_model_default_is_saved_as_none_and_reads_back_as_null() {
        val none: ThinkingEffort? = null
        assertEquals("none", none.storedText())
        assertNull(defaultEffortNamed(none.storedText()))
    }
}
