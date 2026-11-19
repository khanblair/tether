package app.tether.updates

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VersionsTest {
    @Test
    fun a_version_parses_with_or_without_v() {
        assertEquals(Triple(0L, 0L, 3L), parseVersion("0.0.3"))
        assertEquals(Triple(1L, 22L, 333L), parseVersion("v1.22.333"))
    }

    @Test
    fun anything_else_does_not_parse() {
        for (text in listOf(
            "",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "1.2.3-rc1",
            "v",
            "1..3",
            "-1.2.3",
            " 1.2.3",
        )) {
            assertNull(text, parseVersion(text))
        }
    }

    @Test
    fun versions_compare_as_numbers_not_text() {
        assertTrue(isNewerVersion("0.0.10", "0.0.9"))
        assertTrue(isNewerVersion("1.0.0", "0.99.99"))
        assertTrue(isNewerVersion("0.1.0", "0.0.99"))
        assertFalse(isNewerVersion("0.0.9", "0.0.10"))
    }

    @Test
    fun an_equal_or_unparseable_version_is_never_newer() {
        assertFalse(isNewerVersion("0.0.2", "0.0.2"))
        assertFalse(isNewerVersion("nonsense", "0.0.2"))
        assertFalse(isNewerVersion("0.0.3", "nonsense"))
    }
}
