package app.tether.newsession

import org.junit.Assert.assertEquals
import org.junit.Test

class NewSessionDetailsTest {
    @Test
    fun detail_count_chooses_none_one_or_many() {
        assertEquals(DetailCount.None, detailCount(0))
        assertEquals(DetailCount.One, detailCount(1))
        assertEquals(DetailCount.Many, detailCount(2))
    }

    @Test
    fun instructions_detail_distinguishes_file_and_missing_file() {
        assertEquals(false, instructionsDetail(null))
        assertEquals(true, instructionsDetail("AGENTS.md"))
    }
}
