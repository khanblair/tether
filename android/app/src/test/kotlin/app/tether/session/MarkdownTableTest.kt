package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MarkdownTableTest {
    @Test fun splitsCellsBetweenOuterPipes() {
        assertEquals(listOf("a", "b", "c"), splitTableRow("| a | b | c |"))
    }

    @Test fun splitsRowWithoutOuterPipes() {
        assertEquals(listOf("a", "b"), splitTableRow("a | b"))
    }

    @Test fun escapedPipeStaysInsideCell() {
        assertEquals(listOf("a | b", "c"), splitTableRow("| a \\| b | c |"))
    }

    @Test fun emptyCellsKeepTheirPlace() {
        assertEquals(listOf("", "x"), splitTableRow("| | x |"))
    }

    @Test fun parsesHeaderDelimiterAndBodyRows() {
        val lines = listOf("| # | Name |", "|---|------|", "| 1 | task |", "| 2 | pouch |")
        val parse = requireNotNull(parseTableAt(lines, 0))
        assertEquals(listOf("#", "Name"), parse.table.header)
        assertEquals(listOf(ColumnAlign.START, ColumnAlign.START), parse.table.align)
        assertEquals(listOf(listOf("1", "task"), listOf("2", "pouch")), parse.table.rows)
        assertEquals(4, parse.lineCount)
    }

    @Test fun readsAlignmentFromColons() {
        val lines = listOf("| a | b | c |", "|:---|:---:|---:|")
        val parse = requireNotNull(parseTableAt(lines, 0))
        assertEquals(
            listOf(ColumnAlign.START, ColumnAlign.CENTER, ColumnAlign.END),
            parse.table.align,
        )
    }

    @Test fun bodyStopsAtFirstBlankLine() {
        val lines = listOf("| a |", "|---|", "| 1 |", "", "| 2 |")
        val parse = requireNotNull(parseTableAt(lines, 0))
        assertEquals(listOf(listOf("1")), parse.table.rows)
        assertEquals(3, parse.lineCount)
    }

    @Test fun bodyStopsAtFirstLineWithoutPipe() {
        val lines = listOf("| a |", "|---|", "| 1 |", "text", "| 2 |")
        val parse = requireNotNull(parseTableAt(lines, 0))
        assertEquals(listOf(listOf("1")), parse.table.rows)
        assertEquals(3, parse.lineCount)
    }

    @Test fun headerWithoutDelimiterRowIsNotATable() {
        assertNull(parseTableAt(listOf("| a | b |", "| 1 | 2 |"), 0))
    }

    @Test fun delimiterWithDifferentCellCountIsNotATable() {
        assertNull(parseTableAt(listOf("| a | b |", "|---|---|---|"), 0))
    }

    @Test fun headerWithoutPipeIsNotATable() {
        assertNull(parseTableAt(listOf("a b", "|---|"), 0))
    }

    @Test fun delimiterWithFewerThanThreeDashesIsNotATable() {
        assertNull(parseTableAt(listOf("| a | b |", "|--|--|"), 0))
    }

    @Test fun startPastTheEndIsNotATable() {
        assertNull(parseTableAt(listOf("| a |", "|---|"), 5))
    }

    @Test fun shortBodyRowIsPaddedWithEmptyCells() {
        val lines = listOf("| a | b | c |", "|---|---|---|", "| 1 |")
        val parse = requireNotNull(parseTableAt(lines, 0))
        assertEquals(listOf(listOf("1", "", "")), parse.table.rows)
    }

    @Test fun longBodyRowIsCutToColumnCount() {
        val lines = listOf("| a | b | c |", "|---|---|---|", "| 1 | 2 | 3 | 4 |")
        val parse = requireNotNull(parseTableAt(lines, 0))
        assertEquals(listOf(listOf("1", "2", "3")), parse.table.rows)
    }

    @Test fun tableAtNonZeroStartWorks() {
        val lines = listOf("intro", "| a |", "|---|", "| 1 |")
        val parse = requireNotNull(parseTableAt(lines, 1))
        assertEquals(listOf("a"), parse.table.header)
        assertEquals(listOf(listOf("1")), parse.table.rows)
        assertEquals(3, parse.lineCount)
    }

    @Test fun aRepoListFromAnAgentReplyBecomesOneTable() {
        val reply =
            """
            Here are 5 of your repositories:

            | # | Name | Description | Visibility | Updated |
            |---|------|-------------|------------|---------|
            | 1 | **task-manager** | A basic to-do list | Public | Nov 27, 2026 |
            | 2 | **pouch** | (no description) | Private | Nov 27, 2026 |
            | 3 | *(6th page needed)* | | | |

            You're signed in as khanblair.
            """.trimIndent()

        val lines = markdownLines(reply)
        val table = requireNotNull(lines.single { it.kind == LineKind.TABLE }.table)

        assertEquals(listOf("#", "Name", "Description", "Visibility", "Updated"), table.header)
        assertEquals(3, table.rows.size)
        assertEquals("**task-manager**", table.rows[0][1])
        assertEquals(listOf("3", "*(6th page needed)*", "", "", ""), table.rows[2])
        assertEquals(LineKind.PLAIN, lines.first().kind)
        assertEquals("You're signed in as khanblair.", lines.last().text.trim())
    }
}
