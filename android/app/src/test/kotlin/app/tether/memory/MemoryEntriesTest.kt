package app.tether.memory

import org.junit.Assert.assertEquals
import org.junit.Test

class MemoryEntriesTest {
    @Test
    fun the_file_splits_into_entries_at_the_section_sign() {
        assertEquals(
            listOf("Use pytest", "Python 3.12"),
            splitEntries("Use pytest\n§\nPython 3.12"),
        )
    }

    @Test
    fun blank_pieces_are_dropped_as_the_core_drops_them() {
        assertEquals(listOf("One"), splitEntries("\n§\nOne\n§\n   "))
    }

    @Test
    fun an_empty_file_has_no_entries() {
        assertEquals(emptyList<String>(), splitEntries(""))
    }

    @Test
    fun characters_are_counted_as_code_points_not_utf16_units() {
        assertEquals(3, characterCount("a😀b"))
    }

    @Test
    fun digits_are_grouped_by_threes() {
        assertEquals("999", groupDigits(999))
        assertEquals("1,240", groupDigits(1240))
        assertEquals("2,200", groupDigits(2200))
        assertEquals("1,000,000", groupDigits(1_000_000))
    }

    @Test
    fun the_meter_shows_the_share_of_the_cap_and_stops_at_full() {
        assertEquals(0.5f, capFraction(1100, 2200), 0.0001f)
        assertEquals(1f, capFraction(3000, 2200), 0.0001f)
        assertEquals(0f, capFraction(0, 2200), 0.0001f)
    }

    @Test
    fun the_caps_match_the_core() {
        assertEquals(2200, MemoryFile.Memory.cap)
        assertEquals(1375, MemoryFile.User.cap)
    }
}
