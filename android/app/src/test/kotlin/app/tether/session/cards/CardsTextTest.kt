package app.tether.session.cards

import app.tether.core.generated.PermissionModeView
import app.tether.session.agent.PlanUi
import app.tether.session.agent.TodoStatus
import app.tether.session.agent.TodoUi
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class CardsTextTest {
    private fun item(
        text: String,
        status: TodoStatus,
    ) = TodoUi(text, status)

    private val seven =
        listOf(
            item("a", TodoStatus.Completed),
            item("b", TodoStatus.Completed),
            item("c", TodoStatus.Completed),
            item("d", TodoStatus.InProgress),
            item("e", TodoStatus.Pending),
            item("f", TodoStatus.Pending),
            item("g", TodoStatus.Pending),
        )

    @Test
    fun countsDoneAndTotal() {
        assertEquals(ChecklistCounts(done = 3, total = 7), checklistCounts(seven))
        assertEquals(ChecklistCounts(done = 0, total = 0), checklistCounts(emptyList()))
    }

    @Test
    fun currentIsFirstInProgressItem() {
        val items =
            listOf(
                item("pending", TodoStatus.Pending),
                item("running", TodoStatus.InProgress),
                item("second running", TodoStatus.InProgress),
            )
        assertEquals("running", currentChecklistItem(items)?.text)
    }

    @Test
    fun currentFallsBackToFirstPendingThenLast() {
        val pendingFirst =
            listOf(
                item("done", TodoStatus.Completed),
                item("next", TodoStatus.Pending),
                item("later", TodoStatus.Pending),
            )
        assertEquals("next", currentChecklistItem(pendingFirst)?.text)

        val allDone =
            listOf(
                item("one", TodoStatus.Completed),
                item("two", TodoStatus.Completed),
            )
        assertEquals("two", currentChecklistItem(allDone)?.text)
        assertNull(currentChecklistItem(emptyList()))
    }

    @Test
    fun allDoneNeedsItemsAndAllCompleted() {
        assertFalse(checklistAllDone(emptyList()))
        assertFalse(checklistAllDone(seven))
        assertTrue(checklistAllDone(listOf(item("x", TodoStatus.Completed))))
    }

    @Test
    fun pinHiddenOnlyWhenDismissedAndAllDone() {
        val done = listOf(item("x", TodoStatus.Completed))
        assertFalse(checklistShown(emptyList(), dismissed = false))
        assertTrue(checklistShown(seven, dismissed = true))
        assertFalse(checklistShown(done, dismissed = true))
        assertTrue(checklistShown(done, dismissed = false))
    }

    @Test
    fun answerIsTrimmedAndCappedAtLimit() {
        assertNull(answerText("   "))
        assertNull(answerText(""))
        assertEquals("hi there", answerText("  hi there  "))
        assertEquals("x".repeat(ANSWER_MAX_CHARS), answerText("x".repeat(600)))
        // The cut at 500 leaves a trailing space, which is trimmed away.
        assertEquals(499, answerText("y".repeat(499) + " z".repeat(10))!!.length)
    }

    @Test
    fun middleTruncationKeepsBothEnds() {
        assertEquals("src/limits.py", truncateMiddle("src/limits.py", 40))
        val cut = truncateMiddle("src/api/search/handler_with_a_long_name.py", 20)
        assertEquals(20, cut.length)
        assertTrue(cut.startsWith("src/api"))
        assertTrue(cut.endsWith("name.py"))
        assertTrue(cut.contains("…"))
    }

    @Test
    fun planTextHasSummaryThenNumberedSteps() {
        val plan =
            PlanUi(
                toolCallId = null,
                summary = "Add a limiter.",
                steps = listOf("Add a bucket.", "Wrap the handler."),
                files = emptyList(),
                commands = emptyList(),
                openQuestions = emptyList(),
            )
        assertEquals("Add a limiter.\n1. Add a bucket.\n2. Wrap the handler.", planAsText(plan))
    }

    @Test
    fun runModesAreAskAcceptEditsAndAutoOnly() {
        assertEquals(
            listOf(
                PermissionModeView.ASK,
                PermissionModeView.ACCEPT_EDITS,
                PermissionModeView.AUTO,
            ),
            RUN_PLAN_MODES,
        )
    }
}
