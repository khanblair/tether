package app.tether.share

import kotlin.test.Test
import kotlin.test.assertNull

class ShareInboxTest {
    @Test
    fun trimsCapsAndResetsProjectWhenAnotherShareArrives() {
        val inbox = ShareInbox()
        inbox.acceptText("  first  ")
        inbox.chooseProject("project-1")
        inbox.acceptText("x".repeat(40_000))
        assertNull(inbox.pending.value?.projectId)
        kotlin.test.assertEquals(32_768, inbox.pending.value?.value?.length)
        inbox.clear()
        assertNull(inbox.pending.value)
    }
}
