package app.tether.share

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ShareInboxTest {
    @Test
    fun trimsCapsAndResetsProjectWhenAnotherShareArrives() {
        val inbox = ShareInbox()
        inbox.acceptText("  first  ")
        inbox.chooseProject("project-1")
        inbox.acceptText("x".repeat(40_000))
        assertNull(inbox.pending.value?.projectId)
        assertEquals(
            32_768,
            inbox.pending.value
                ?.value
                ?.length,
        )
        inbox.clear()
        assertNull(inbox.pending.value)
    }
}
