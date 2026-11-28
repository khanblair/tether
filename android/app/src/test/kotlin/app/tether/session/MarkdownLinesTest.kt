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

    @Test fun deepHeadingsShareTheLevelThreeStyleAndTooManyHashesStayPlain() {
        assertEquals(MarkdownLine(LineKind.HEADING_3, "four"), classifyLine("#### four"))
        assertEquals(MarkdownLine(LineKind.HEADING_3, "six"), classifyLine("###### six"))
        assertEquals(MarkdownLine(LineKind.PLAIN, "####### seven"), classifyLine("####### seven"))
        assertEquals(LineKind.PLAIN, classifyLine("#tag").kind)
    }

    @Test fun dashAndStarBulletsBecomeBullets() {
        assertEquals(MarkdownLine(LineKind.BULLET, "one"), classifyLine("- one"))
        assertEquals(MarkdownLine(LineKind.BULLET, "two"), classifyLine("* two"))
    }

    @Test fun plusBulletBecomesBullet() {
        assertEquals(MarkdownLine(LineKind.BULLET, "plus"), classifyLine("+ plus"))
    }

    @Test fun twoLeadingSpacesGiveLevelOne() {
        assertEquals(
            MarkdownLine(LineKind.BULLET, "nested", level = 1),
            classifyLine("  - nested"),
        )
    }

    @Test fun fourLeadingSpacesGiveLevelTwo() {
        assertEquals(
            MarkdownLine(LineKind.BULLET, "deeper", level = 2),
            classifyLine("    - deeper"),
        )
    }

    @Test fun tabThenDashGivesLevelOne() {
        assertEquals(MarkdownLine(LineKind.BULLET, "x", level = 1), classifyLine("\t- x"))
    }

    @Test fun tenLeadingSpacesAreCappedAtMaxLevel() {
        assertEquals(
            MarkdownLine(LineKind.BULLET, "x", level = MAX_LEVEL),
            classifyLine("          - x"),
        )
        assertEquals(3, MAX_LEVEL)
    }

    @Test fun emphasisAtLineStartIsNotABullet() {
        assertEquals(LineKind.PLAIN, classifyLine("*em* text").kind)
    }

    @Test fun streamingBulletWithoutTextIsAnEmptyBullet() {
        assertEquals(MarkdownLine(LineKind.BULLET, ""), classifyLine("- "))
    }

    @Test fun numberedItemKeepsItsNumber() {
        assertEquals(MarkdownLine(LineKind.NUMBERED, "third", "3."), classifyLine("3. third"))
    }

    @Test fun indentedNumberedItemKeepsNumberAndLevel() {
        assertEquals(
            MarkdownLine(LineKind.NUMBERED, "nested", marker = "2.", level = 1),
            classifyLine("  2. nested"),
        )
    }

    @Test fun parenthesisNumberedItemUsesDotMarker() {
        assertEquals(
            MarkdownLine(LineKind.NUMBERED, "paren", marker = "3.", level = 0),
            classifyLine("3) paren"),
        )
    }

    @Test fun thematicBreaksAreRules() {
        assertEquals(LineKind.RULE, classifyLine("---").kind)
        assertEquals(LineKind.RULE, classifyLine("***").kind)
        assertEquals(LineKind.RULE, classifyLine("___").kind)
        assertEquals(LineKind.RULE, classifyLine("- - -").kind)
        assertEquals(LineKind.RULE, classifyLine("* * *").kind)
    }

    @Test fun twoDashesStayPlain() {
        assertEquals(LineKind.PLAIN, classifyLine("--").kind)
    }

    @Test fun dashItemStaysBullet() {
        assertEquals(MarkdownLine(LineKind.BULLET, "item"), classifyLine("- item"))
    }

    @Test fun boldAtLineStartStaysPlain() {
        assertEquals(LineKind.PLAIN, classifyLine("**bold** text").kind)
    }

    @Test fun quoteLineIsQuote() {
        assertEquals(MarkdownLine(LineKind.QUOTE, "quoted"), classifyLine("> quoted"))
    }

    @Test fun quoteWithoutSpaceIsQuote() {
        assertEquals(MarkdownLine(LineKind.QUOTE, "no space"), classifyLine(">no space"))
    }

    @Test fun bareQuoteMarkIsEmptyQuote() {
        assertEquals(MarkdownLine(LineKind.QUOTE, ""), classifyLine(">"))
    }

    @Test fun consecutiveQuoteLinesMergeIntoOneQuote() {
        assertEquals(
            listOf(MarkdownLine(LineKind.QUOTE, "a\nb")),
            markdownLines("> a\n> b"),
        )
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

    @Test fun tableBetweenProseLinesBecomesOneTableLine() {
        val table =
            MarkdownTable(
                header = listOf("a", "b"),
                align = listOf(ColumnAlign.START, ColumnAlign.START),
                rows = listOf(listOf("1", "2")),
            )
        assertEquals(
            listOf(
                MarkdownLine(LineKind.PLAIN, "intro"),
                MarkdownLine(LineKind.TABLE, "", table = table),
                MarkdownLine(LineKind.PLAIN, "after"),
            ),
            markdownLines("intro\n| a | b |\n|---|---|\n| 1 | 2 |\nafter"),
        )
    }

    @Test fun headerWithoutDelimiterRowStaysPlainLines() {
        assertEquals(
            listOf(MarkdownLine(LineKind.PLAIN, "| a | b |\n| 1 | 2 |")),
            markdownLines("| a | b |\n| 1 | 2 |"),
        )
    }

    @Test fun tableStopsAtHeadingWithoutPipe() {
        val table =
            MarkdownTable(
                header = listOf("a", "b"),
                align = listOf(ColumnAlign.START, ColumnAlign.START),
                rows = listOf(listOf("1", "2")),
            )
        assertEquals(
            listOf(
                MarkdownLine(LineKind.TABLE, "", table = table),
                MarkdownLine(LineKind.HEADING_2, "Next"),
            ),
            markdownLines("| a | b |\n|---|---|\n| 1 | 2 |\n## Next"),
        )
    }
}
