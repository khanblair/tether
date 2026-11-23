package app.tether.session.agent

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ToolCallsFromMessagesTest {
    private fun message(
        id: String,
        role: RoleView,
        vararg parts: MessagePartView,
    ) = MessageView(
        id = id,
        sessionId = "s1",
        role = role,
        seq = 1u,
        createdAtMs = 0L,
        rewound = false,
        parts = parts.toList(),
    )

    private fun call(
        id: String,
        tool: String = "read_file",
    ) = MessagePartView.ToolCall(id, tool, "{}")

    private fun result(
        id: String,
        text: String,
        isError: Boolean = false,
        resultRef: String? = null,
    ) = MessagePartView.ToolResult(id, isError, text, resultRef)

    @Test
    fun aSucceededCallTakesItsOutputFromItsResult() {
        val calls =
            toolCallsFromMessages(
                listOf(
                    message("m1", RoleView.ASSISTANT, call("t1")),
                    message("m2", RoleView.TOOL, result("t1", "file body")),
                ),
            )

        assertEquals(
            mapOf(
                "t1" to ToolCallUi("t1", "read_file", "", ToolStatus.Succeeded, "file body", false),
            ),
            calls,
        )
    }

    @Test
    fun anErrorResultMakesTheCallFailed() {
        val calls =
            toolCallsFromMessages(
                listOf(
                    message("m1", RoleView.ASSISTANT, call("t1", "run")),
                    message("m2", RoleView.TOOL, result("t1", "exit 1", isError = true)),
                ),
            )

        assertEquals(ToolStatus.Failed, calls.getValue("t1").status)
        assertEquals("exit 1", calls.getValue("t1").output)
    }

    @Test
    fun aCallWithoutAResultIsCancelled() {
        val calls = toolCallsFromMessages(listOf(message("m1", RoleView.ASSISTANT, call("t1"))))

        assertEquals(ToolStatus.Cancelled, calls.getValue("t1").status)
        assertEquals("", calls.getValue("t1").output)
    }

    @Test
    fun twoCallsInOneMessageKeepTheirOrderAndTheirOwnResults() {
        val calls =
            toolCallsFromMessages(
                listOf(
                    message("m1", RoleView.ASSISTANT, call("t1"), call("t2")),
                    message(
                        "m2",
                        RoleView.TOOL,
                        result("t2", "second"),
                        result("t1", "first"),
                    ),
                ),
            )

        assertEquals(listOf("t1", "t2"), calls.keys.toList())
        assertEquals("first", calls.getValue("t1").output)
        assertEquals("second", calls.getValue("t2").output)
    }

    @Test
    fun aResultCutInTheStoreIsMarkedTruncated() {
        val calls =
            toolCallsFromMessages(
                listOf(
                    message("m1", RoleView.ASSISTANT, call("t1")),
                    message("m2", RoleView.TOOL, result("t1", "head", resultRef = "abc123")),
                ),
            )

        assertTrue(calls.getValue("t1").outputTruncated)
    }

    @Test
    fun aResultWithoutAMatchingCallIsIgnored() {
        val calls =
            toolCallsFromMessages(
                listOf(message("m1", RoleView.TOOL, result("ghost", "orphan"))),
            )

        assertTrue(calls.isEmpty())
    }

    @Test
    fun theFirstCallWithAnIdWins() {
        val calls =
            toolCallsFromMessages(
                listOf(
                    message("m1", RoleView.ASSISTANT, call("t1", "read_file")),
                    message("m2", RoleView.ASSISTANT, call("t1", "glob")),
                ),
            )

        assertEquals("read_file", calls.getValue("t1").tool)
    }

    @Test
    fun segmentsGroupRunsOfSavedCallsAndDropResults() {
        val persisted =
            toolCallsFromMessages(
                listOf(
                    message(
                        "m1",
                        RoleView.ASSISTANT,
                        MessagePartView.Text("Looking."),
                        call("t1"),
                        call("t2"),
                        MessagePartView.Text("Done."),
                    ),
                    message("m2", RoleView.TOOL, result("t1", "a"), result("t2", "b")),
                ),
            )
        val parts =
            listOf(
                MessagePartView.Text("Looking."),
                call("t1"),
                call("t2"),
                MessagePartView.Text("Done."),
            )

        val segments = segmentParts(parts, persisted)

        assertEquals(
            listOf(
                PartSegment.Part(MessagePartView.Text("Looking.")),
                PartSegment.Calls(persisted.values.toList()),
                PartSegment.Part(MessagePartView.Text("Done.")),
            ),
            segments,
        )
    }

    @Test
    fun aLiveCallIsLeftOutOfTheMessage() {
        val segments = segmentParts(listOf(call("live")), persisted = emptyMap())

        assertTrue(segments.isEmpty())
    }

    @Test
    fun aToolMessageWithOnlyResultsIsHidden() {
        val resultsOnly = message("m2", RoleView.TOOL, result("t1", "a"))

        assertTrue(resultsOnly.isToolResultOnly())
        assertTrue(!message("m3", RoleView.ASSISTANT, call("t1")).isToolResultOnly())
    }
}
