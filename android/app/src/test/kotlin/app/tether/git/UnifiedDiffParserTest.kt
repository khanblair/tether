package app.tether.git

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Diff text here is what Git and the Rust core print, copied from real `git diff` output. */
class UnifiedDiffParserTest {
    private fun text(vararg lines: String): String = lines.joinToString("\n") + "\n"

    private val modifiedAndBinary =
        text(
            "diff --git a/a.txt b/a.txt",
            "index 0c2aa38..6070130 100644",
            "--- a/a.txt",
            "+++ b/a.txt",
            "@@ -1,3 +1,4 @@",
            " line one",
            "-line two",
            "+line TWO",
            " line three",
            "+line four",
            "diff --git a/b.bin b/b.bin",
            "index b1feab4..4e7db97 100644",
            "Binary files a/b.bin and b/b.bin differ",
        )

    @Test
    fun counts_added_and_removed_lines_and_numbers_them() {
        val file = UnifiedDiffParser.parse(modifiedAndBinary).first()

        assertEquals("a.txt", file.path)
        assertNull(file.oldPath)
        assertEquals(2, file.added)
        assertEquals(1, file.removed)
        assertFalse(file.binary)
        val lines = file.hunks.single().lines
        assertEquals(
            listOf(
                DiffLineModel(DiffLineKind.Context, 1, 1, "line one"),
                DiffLineModel(DiffLineKind.Removed, 2, null, "line two"),
                DiffLineModel(DiffLineKind.Added, null, 2, "line TWO"),
                DiffLineModel(DiffLineKind.Context, 3, 3, "line three"),
                DiffLineModel(DiffLineKind.Added, null, 4, "line four"),
            ),
            lines,
        )
    }

    @Test
    fun keeps_the_whole_hunk_header_with_its_function_name() {
        val diff =
            text(
                "diff --git a/src/auth/session.py b/src/auth/session.py",
                "--- a/src/auth/session.py",
                "+++ b/src/auth/session.py",
                "@@ -41,7 +41,7 @@ is_expired",
                " def is_expired(token, now):",
                "-    return token.expires_at < now",
                "+    return token.expires_at <= now",
            )

        val hunk =
            UnifiedDiffParser
                .parse(diff)
                .single()
                .hunks
                .single()

        assertEquals("@@ -41,7 +41,7 @@ is_expired", hunk.header)
        assertEquals(DiffLineKind.Context, hunk.lines[0].kind)
        assertEquals(41, hunk.lines[0].oldNo)
    }

    @Test
    fun reads_every_file_of_a_multi_file_diff_in_order() {
        val diff =
            text(
                "diff --git a/a.txt b/a.txt",
                "--- a/a.txt",
                "+++ b/a.txt",
                "@@ -1 +1 @@",
                "-x",
                "+y",
                "diff --git a/b.bin b/b.bin",
                "Binary files a/b.bin and b/b.bin differ",
                "diff --git a/new.txt b/new.txt",
                "new file mode 100644",
                "index 0000000..fa49b07",
                "--- /dev/null",
                "+++ b/new.txt",
                "@@ -0,0 +1 @@",
                "+new file",
            )

        val files = UnifiedDiffParser.parse(diff)

        assertEquals(listOf("a.txt", "b.bin", "new.txt"), files.map { it.path })
    }

    @Test
    fun marks_a_binary_file_and_gives_it_no_hunks() {
        val binary = UnifiedDiffParser.parse(modifiedAndBinary)[1]

        assertEquals("b.bin", binary.path)
        assertTrue(binary.binary)
        assertTrue(binary.hunks.isEmpty())
        assertEquals(0, binary.added + binary.removed)
    }

    @Test
    fun reads_a_new_file_with_no_old_line_numbers() {
        val diff =
            text(
                "diff --git a/new.txt b/new.txt",
                "new file mode 100644",
                "index 0000000..fa49b07",
                "--- /dev/null",
                "+++ b/new.txt",
                "@@ -0,0 +1 @@",
                "+new file",
            )

        val file = UnifiedDiffParser.parse(diff).single()

        assertEquals("new.txt", file.path)
        assertEquals(1, file.added)
        assertEquals(
            listOf(DiffLineModel(DiffLineKind.Added, null, 1, "new file")),
            file.hunks.single().lines,
        )
    }

    @Test
    fun reads_a_deleted_file_with_its_old_path_and_no_new_numbers() {
        val diff =
            text(
                "diff --git a/del.txt b/del.txt",
                "deleted file mode 100644",
                "index c338b89..0000000",
                "--- a/del.txt",
                "+++ /dev/null",
                "@@ -1,2 +0,0 @@",
                "-gone",
                "-stays",
            )

        val file = UnifiedDiffParser.parse(diff).single()

        assertEquals("del.txt", file.path)
        assertEquals(2, file.removed)
        assertEquals(
            listOf(
                DiffLineModel(DiffLineKind.Removed, 1, null, "gone"),
                DiffLineModel(DiffLineKind.Removed, 2, null, "stays"),
            ),
            file.hunks.single().lines,
        )
    }

    @Test
    fun reads_a_rename_from_its_rename_lines() {
        val diff =
            text(
                "diff --git a/renamed.txt b/moved.txt",
                "similarity index 100%",
                "rename from renamed.txt",
                "rename to moved.txt",
            )

        val file = UnifiedDiffParser.parse(diff).single()

        assertEquals("moved.txt", file.path)
        assertEquals("renamed.txt", file.oldPath)
        assertTrue(file.hunks.isEmpty())
        assertFalse(file.binary)
    }

    @Test
    fun attaches_the_no_newline_marker_as_a_note_line() {
        val diff =
            text(
                "diff --git a/nonl.txt b/nonl.txt",
                "--- a/nonl.txt",
                "+++ b/nonl.txt",
                "@@ -1 +1 @@",
                "-no newline",
                "\\ No newline at end of file",
                "+no newline changed",
                "\\ No newline at end of file",
            )

        val lines =
            UnifiedDiffParser
                .parse(diff)
                .single()
                .hunks
                .single()
                .lines

        assertEquals(
            listOf(
                DiffLineKind.Removed,
                DiffLineKind.Note,
                DiffLineKind.Added,
                DiffLineKind.Note,
            ),
            lines.map { it.kind },
        )
        assertEquals("No newline at end of file", lines[1].text)
        assertNull(lines[1].oldNo)
        assertNull(lines[1].newNo)
    }

    @Test
    fun strips_carriage_returns_from_crlf_content() {
        val diff =
            text(
                "diff --git a/crlf.txt b/crlf.txt",
                "--- a/crlf.txt",
                "+++ b/crlf.txt",
                "@@ -1,2 +1,2 @@",
                " x\r",
                "-y\r",
                "+Y\r",
            )

        val lines =
            UnifiedDiffParser
                .parse(diff)
                .single()
                .hunks
                .single()
                .lines

        assertEquals(listOf("x", "y", "Y"), lines.map { it.text })
    }

    @Test
    fun gives_a_mode_only_change_no_hunks_and_not_binary() {
        val diff =
            text(
                "diff --git a/mode.sh b/mode.sh",
                "old mode 100644",
                "new mode 100755",
            )

        val file = UnifiedDiffParser.parse(diff).single()

        assertEquals("mode.sh", file.path)
        assertTrue(file.hunks.isEmpty())
        assertFalse(file.binary)
    }

    @Test
    fun removes_the_tab_git_adds_after_a_name_with_spaces() {
        val diff =
            text(
                "diff --git a/my file.txt b/my file.txt",
                "index 2fa992c..fe5841d 100644",
                "--- a/my file.txt\t",
                "+++ b/my file.txt\t",
                "@@ -1 +1,2 @@",
                " keep",
                "+more",
            )

        assertEquals("my file.txt", UnifiedDiffParser.parse(diff).single().path)
    }

    @Test
    fun unquotes_names_git_quotes_for_special_characters() {
        val diff =
            text(
                "diff --git \"a/caf\\303\\251.txt\" \"b/caf\\303\\251.txt\"",
                "index f2ad6c7..9f853d3 100644",
                "--- \"a/caf\\303\\251.txt\"",
                "+++ \"b/caf\\303\\251.txt\"",
                "@@ -1 +1,2 @@",
                " c",
                "+z",
                "diff --git \"a/q\\\"x.txt\" \"b/q\\\"x.txt\"",
                "--- \"a/q\\\"x.txt\"",
                "+++ \"b/q\\\"x.txt\"",
                "@@ -1 +1,2 @@",
                " a",
                "+z",
                "diff --git \"a/tab\\tx\" \"b/tab\\tx\"",
                "--- \"a/tab\\tx\"",
                "+++ \"b/tab\\tx\"",
                "@@ -1 +1,2 @@",
                " b",
                "+q",
            )

        val paths = UnifiedDiffParser.parse(diff).map { it.path }

        assertEquals(listOf("café.txt", "q\"x.txt", "tab\tx"), paths)
    }

    @Test
    fun reads_a_hunk_count_left_out_as_one_line() {
        val diff =
            text(
                "diff --git a/one.txt b/one.txt",
                "--- a/one.txt",
                "+++ b/one.txt",
                "@@ -5 +5 @@",
                "-old",
                "+new",
            )

        val lines =
            UnifiedDiffParser
                .parse(diff)
                .single()
                .hunks
                .single()
                .lines

        assertEquals(
            listOf(
                DiffLineModel(DiffLineKind.Removed, 5, null, "old"),
                DiffLineModel(DiffLineKind.Added, null, 5, "new"),
            ),
            lines,
        )
    }

    @Test
    fun treats_a_removed_line_that_starts_with_dashes_as_content() {
        val diff =
            text(
                "diff --git a/c.txt b/c.txt",
                "--- a/c.txt",
                "+++ b/c.txt",
                "@@ -1 +0,0 @@",
                "--- old heading",
            )

        val file = UnifiedDiffParser.parse(diff).single()

        assertEquals(1, file.removed)
        assertEquals(
            "-- old heading",
            file.hunks
                .single()
                .lines
                .single()
                .text,
        )
    }

    @Test
    fun reports_the_truncation_marker_and_keeps_it_out_of_the_lines() {
        val diff =
            text(
                "diff --git a/big.txt b/big.txt",
                "--- a/big.txt",
                "+++ b/big.txt",
                "@@ -1 +1 @@",
                "-old",
                "+new",
                "[diff truncated at 64 KiB]",
            )

        val files = UnifiedDiffParser.parse(diff)

        assertTrue(UnifiedDiffParser.isTruncated(diff))
        assertEquals(
            2,
            files
                .single()
                .hunks
                .single()
                .lines.size,
        )
    }

    @Test
    fun a_diff_without_the_marker_is_not_truncated() {
        assertFalse(UnifiedDiffParser.isTruncated(modifiedAndBinary))
    }

    @Test
    fun an_empty_string_gives_no_files() {
        assertTrue(UnifiedDiffParser.parse("").isEmpty())
        assertFalse(UnifiedDiffParser.isTruncated(""))
    }

    @Test
    fun garbage_gives_no_files_and_never_throws() {
        val garbage = "hello\nworld\n@@ nonsense\n+x\n-y\n\u0000\u0001"

        assertTrue(UnifiedDiffParser.parse(garbage).isEmpty())
    }

    @Test
    fun unquote_leaves_plain_text_alone_and_decodes_quoted_text() {
        assertEquals("a/plain.txt", unquote("a/plain.txt"))
        assertEquals("\"", unquote("\"\\\"\""))
        assertEquals("a\tb", unquote("\"a\\tb\""))
        assertEquals("\"", unquote("\""))
    }
}
