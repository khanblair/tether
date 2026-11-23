package app.tether.git

import org.junit.Assert.assertEquals
import org.junit.Test

class DiffRowTest {
    private val text =
        "diff --git a/a.txt b/a.txt\n" +
            "--- a/a.txt\n" +
            "+++ b/a.txt\n" +
            "@@ -1 +1 @@\n" +
            "-old\n" +
            "+new\n" +
            "diff --git a/b.bin b/b.bin\n" +
            "Binary files a/b.bin and b/b.bin differ\n" +
            "diff --git a/mode.sh b/mode.sh\n" +
            "old mode 100644\n" +
            "new mode 100755\n"

    @Test
    fun flattens_each_file_into_its_header_body_and_lines_in_order() {
        val rows = flattenDiff(UnifiedDiffParser.parse(text))

        assertEquals(
            listOf(
                "FileHeader:0",
                "HunkHeader",
                "Line:Removed",
                "Line:Added",
                "FileHeader:1",
                "Binary:1",
                "FileHeader:2",
                "NoText:2",
            ),
            rows.map { row ->
                when (row) {
                    is DiffRow.FileHeader -> "FileHeader:${row.fileIndex}"
                    is DiffRow.HunkHeader -> "HunkHeader"
                    is DiffRow.Line -> "Line:${row.line.kind.name}"
                    is DiffRow.Binary -> "Binary:${row.fileIndex}"
                    is DiffRow.NoText -> "NoText:${row.fileIndex}"
                }
            },
        )
    }

    @Test
    fun finds_the_row_where_a_file_starts() {
        val rows = flattenDiff(UnifiedDiffParser.parse(text))

        assertEquals(0, rowIndexOfFile(rows, 0))
        assertEquals(4, rowIndexOfFile(rows, 1))
        assertEquals(-1, rowIndexOfFile(rows, 9))
    }

    @Test
    fun no_files_give_no_rows() {
        assertEquals(emptyList<DiffRow>(), flattenDiff(emptyList()))
    }
}
