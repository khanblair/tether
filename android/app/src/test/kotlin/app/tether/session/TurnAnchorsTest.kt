package app.tether.session

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TurnAnchorsTest {
    private val toolCall = MessagePartView.ToolCall("t1", "read", "{}")
    private val toolResult = MessagePartView.ToolResult("t1", false, "file body", null)

    private fun message(
        id: String,
        role: RoleView,
        vararg parts: MessagePartView,
    ): MessageView =
        MessageView(
            id = id,
            sessionId = "s1",
            role = role,
            seq = 1u,
            createdAtMs = 0L,
            rewound = false,
            parts = parts.toList(),
        )

    private fun user(
        id: String,
        text: String,
    ): MessageView = message(id, RoleView.USER, MessagePartView.Text(text))

    private fun reply(
        id: String,
        text: String,
    ): MessageView = message(id, RoleView.ASSISTANT, MessagePartView.Text(text))

    @Test fun turnTextSkipsThinkingAndToolCalls() {
        val messages =
            listOf(
                user("u1", "Explain"),
                message(
                    "a1",
                    RoleView.ASSISTANT,
                    MessagePartView.Thinking("private reasoning"),
                    MessagePartView.Text("Answer A"),
                ),
                message("t1m", RoleView.TOOL, toolCall, toolResult),
                reply("a2", "Done"),
            )

        val anchors = turnAnchors(messages, running = false)

        assertEquals("Answer A\n\nDone", anchors.getValue("a2").replyText)
    }

    @Test fun multiStepTurnAnchorsOnlyItsLastAssistantMessage() {
        val messages =
            listOf(
                user("u1", "Fix it"),
                reply("a1", "Looking"),
                message("t1m", RoleView.TOOL, toolCall, toolResult),
                reply("a2", "Editing"),
                message("t2m", RoleView.TOOL, toolCall, toolResult),
                reply("a3", "Fixed"),
            )

        val anchors = turnAnchors(messages, running = false)

        assertEquals(setOf("a3"), anchors.keys)
        assertTrue(anchors.getValue("a3").newest)
    }

    @Test fun onlyTheTurnAtTheEndIsNewest() {
        val messages =
            listOf(
                user("u1", "First"),
                reply("a1", "One"),
                user("u2", "Second"),
                reply("a2", "Two"),
            )

        val anchors = turnAnchors(messages, running = false)

        assertFalse(anchors.getValue("a1").newest)
        assertTrue(anchors.getValue("a2").newest)
    }

    @Test fun runningTurnGetsNoAnchorButEarlierTurnsKeepTheirs() {
        val messages =
            listOf(
                user("u1", "First"),
                reply("a1", "One"),
                user("u2", "Second"),
                reply("a2", "Streaming"),
            )

        val anchors = turnAnchors(messages, running = true)

        assertEquals(setOf("a1"), anchors.keys)
        assertFalse(anchors.getValue("a1").newest)
    }

    @Test fun transcriptEndingWithUserHasNoNewestTurn() {
        val messages = listOf(reply("a1", "One"), user("u1", "Next"))

        val anchors = turnAnchors(messages, running = false)

        assertEquals(setOf("a1"), anchors.keys)
        assertFalse(anchors.getValue("a1").newest)
    }

    @Test fun systemNoteEndsTheRunAndTakesNoNewestFlag() {
        val note = message("n1", RoleView.SYSTEM_NOTE, MessagePartView.Text("Session stopped."))
        val messages = listOf(user("u1", "Go"), reply("a1", "Done"), note)

        val anchors = turnAnchors(messages, running = false)

        assertFalse(anchors.getValue("a1").newest)
    }

    @Test fun turnWithOnlyToolMessagesHasNoAnchor() {
        val tool = message("t1m", RoleView.TOOL, toolCall, toolResult)
        val messages = listOf(user("u1", "Run"), tool)

        assertTrue(turnAnchors(messages, running = false).isEmpty())
    }

    @Test fun replyCopyCarriesOnlyTheReplyText() {
        val anchors = turnAnchors(listOf(reply("a1", "Only this")), running = false)
        val text = anchors.getValue("a1").replyText
        val copied = reply("a1", "ignored")
        val replyOnly = copied.copy(parts = listOf(MessagePartView.Text(text)))

        assertEquals("Only this", plainText(replyOnly))
    }
}
