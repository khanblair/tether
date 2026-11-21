package app.tether.session

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MessageTextTest {
    private fun message(vararg parts: MessagePartView) =
        MessageView(
            id = "m1",
            sessionId = "s1",
            role = RoleView.ASSISTANT,
            seq = 1u,
            createdAtMs = 0L,
            rewound = false,
            parts = parts.toList(),
        )

    @Test fun textPartsAreJoinedByABlankLine() {
        val message =
            message(MessagePartView.Text("First paragraph."), MessagePartView.Text("Second."))

        assertEquals("First paragraph.\n\nSecond.", plainText(message))
    }

    @Test fun partsThatAreNotTextAreLeftOutOfTheCopy() {
        val message =
            message(MessagePartView.Thinking("private reasoning"), MessagePartView.Text("Answer."))

        assertEquals("Answer.", plainText(message))
    }

    @Test fun aMessageWithNoTextCopiesNothing() {
        assertEquals("", plainText(message(MessagePartView.Thinking("only reasoning"))))
    }

    @Test fun androidTwelveShowsTetherOwnCopiedToast() {
        assertTrue(showsCopiedToast(sdkInt = 32))
    }

    @Test fun androidThirteenLeavesTheCopiedToastToTheSystem() {
        assertFalse(showsCopiedToast(sdkInt = 33))
    }
}
