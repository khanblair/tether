package app.tether.preview

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

class InlinePreviewsTest {
    @Test fun aPutHtmlComesBackUnderItsId() {
        val previews = InlinePreviews()
        val id = previews.put("<h1>one</h1>")
        assertEquals("<h1>one</h1>", previews.get(id))
    }

    @Test fun eachPutGetsItsOwnId() {
        val previews = InlinePreviews()
        val first = previews.put("a")
        val second = previews.put("b")
        assertNotEquals(first, second)
        assertEquals("a", previews.get(first))
        assertEquals("b", previews.get(second))
    }

    @Test fun anUnknownIdGivesNothing() {
        assertNull(InlinePreviews().get("missing"))
    }

    @Test fun itKeepsTheLastEightAndDropsTheNinth() {
        val previews = InlinePreviews()
        val ids = (1..8).map { previews.put("page $it") }
        assertEquals("page 1", previews.get(ids.first()))
        val ninth = previews.put("page 9")
        assertNull(previews.get(ids.first()))
        assertEquals("page 2", previews.get(ids[1]))
        assertEquals("page 9", previews.get(ninth))
    }

    @Test fun theCapacityIsEightByDefault() {
        assertEquals(8, INLINE_PREVIEW_CAPACITY)
        val previews = InlinePreviews()
        val ids = (1..8).map { previews.put("page $it") }
        assertNotNull(previews.get(ids.last()))
    }
}
