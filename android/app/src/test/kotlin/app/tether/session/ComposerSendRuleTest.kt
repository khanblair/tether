package app.tether.session

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ComposerSendRuleTest {
    @Test
    fun textCanSendWhenNothingBlocksIt() {
        assertTrue(composerCanSend("hello", image = null, sendBlocked = false))
    }

    @Test
    fun anApprovalBlocksTheSend() {
        assertFalse(composerCanSend("hello", image = null, sendBlocked = true))
    }

    @Test
    fun blankTextOrAStagedImageKeepsSendOff() {
        assertFalse(composerCanSend("  ", image = null, sendBlocked = false))
        assertFalse(composerCanSend("hello", image = "content://img", sendBlocked = false))
    }
}
