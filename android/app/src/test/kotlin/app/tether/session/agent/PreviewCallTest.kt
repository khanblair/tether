package app.tether.session.agent

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PreviewCallTest {
    private fun call(
        tool: String,
        argsJson: String,
        id: String = "c1",
    ) = MessagePartView.ToolCall(id, tool, argsJson)

    @Test fun aWriteOfAnHtmlPageGivesItsPath() {
        assertEquals("index.html", previewPathOf(call("write_file", """{"path":"index.html"}""")))
    }

    @Test fun aSavedEditOfAPageGivesItsPath() {
        val args = """{"path":"docs/page.htm","old":"a","new":"b"}"""
        assertEquals("docs/page.htm", previewPathOf(call("edit_file", args)))
    }

    @Test fun aMultiEditOfAPageIsPreviewable() {
        assertEquals("index.html", previewPathOf(call("multi_edit", """{"path":"index.html"}""")))
    }

    @Test fun aNonHtmlFileGivesNoPreview() {
        assertNull(previewPathOf(call("write_file", """{"path":"style.css"}""")))
        assertNull(previewPathOf(call("edit_file", """{"path":"notes.txt"}""")))
    }

    @Test fun aReadOrOtherToolGivesNoPreview() {
        assertNull(previewPathOf(call("read_file", """{"path":"index.html"}""")))
    }

    @Test fun aCallWithoutAPathGivesNoPreview() {
        assertNull(previewPathOf(call("write_file", "{}")))
    }

    @Test fun anEscapedSlashIsReadAsASlash() {
        assertEquals("a/b.html", previewPathOf(call("write_file", """{"path":"a\/b.html"}""")))
    }

    @Test fun aPathInsideTheContentIsNotTakenAsThePath() {
        val args = """{"content":"\"path\": \"x.html\"","path":"real.html"}"""
        assertEquals("real.html", previewPathOf(call("write_file", args)))
    }

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

    @Test fun theSavedPathsCoverOnlyThePagesThatWereWritten() {
        val messages =
            listOf(
                message(
                    "m1",
                    RoleView.ASSISTANT,
                    call("write_file", """{"path":"index.html"}"""),
                    call("write_file", """{"path":"app.css"}""", id = "c2"),
                ),
            )
        assertEquals(mapOf("c1" to "index.html"), savedPreviewPaths(messages))
    }

    @Test fun theActionOpensThePathThroughTheHandler() {
        val opened = mutableListOf<String>()
        previewAction("index.html") { opened += it }?.invoke()
        assertEquals(listOf("index.html"), opened)
    }

    @Test fun noHandlerOrNoPathGivesNoAction() {
        assertNull(previewAction("index.html", null))
        assertNull(previewAction(null) {})
    }
}
