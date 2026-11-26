package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class MarkdownLinesTest {
    @Test fun headingsOfLevelOneToThreeAreRecognised() {
        assertEquals(LineKind.HEADING_1, classifyLine("# Title").kind)
        assertEquals(LineKind.HEADING_2, classifyLine("## Two").kind)
        assertEquals(LineKind.HEADING_3, classifyLine("### Three").kind)
        assertEquals("Three", classifyLine("### Three").text)
    }

    @Test fun hashesWithoutSpaceAndDeepHeadingsStayPlain() {
        assertEquals(LineKind.PLAIN, classifyLine("#tag").kind)
        assertEquals(LineKind.PLAIN, classifyLine("#### four").kind)
    }

    @Test fun dashAndStarBulletsBecomeBullets() {
        assertEquals(MarkdownLine(LineKind.BULLET, "one"), classifyLine("- one"))
        assertEquals(MarkdownLine(LineKind.BULLET, "two"), classifyLine("* two"))
    }

    @Test fun emphasisAtLineStartIsNotABullet() {
        assertEquals(LineKind.PLAIN, classifyLine("*em* text").kind)
    }

    @Test fun numberedItemKeepsItsNumber() {
        assertEquals(MarkdownLine(LineKind.NUMBERED, "third", "3."), classifyLine("3. third"))
    }

    @Test fun streamingBulletWithoutTextIsAnEmptyBullet() {
        assertEquals(MarkdownLine(LineKind.BULLET, ""), classifyLine("- "))
    }

    @Test fun plainLinesMergeIntoOneRun() {
        assertEquals(
            listOf(MarkdownLine(LineKind.PLAIN, "a\nb\n\nc")),
            markdownLines("a\nb\n\nc"),
        )
    }

    @Test fun headingsAndListsSplitTheRuns() {
        assertEquals(
            listOf(
                MarkdownLine(LineKind.HEADING_2, "Plan"),
                MarkdownLine(LineKind.PLAIN, "Steps:"),
                MarkdownLine(LineKind.BULLET, "read"),
                MarkdownLine(LineKind.NUMBERED, "then write", "2."),
            ),
            markdownLines("## Plan\nSteps:\n- read\n2. then write"),
        )
    }
}
