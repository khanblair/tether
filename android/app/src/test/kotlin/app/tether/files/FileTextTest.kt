package app.tether.files

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FileTextTest {
    @Test
    fun empty_text_has_no_lines() {
        assertEquals(emptyList<String>(), sourceLines(""))
    }

    @Test
    fun a_trailing_line_end_does_not_add_a_line() {
        assertEquals(listOf("a", "b"), sourceLines("a\nb\n"))
        assertEquals(listOf("a", "b"), sourceLines("a\r\nb\r\n"))
        assertEquals(listOf("a", "b"), sourceLines("a\nb"))
    }

    @Test
    fun inner_blank_lines_are_kept_and_crlf_is_one_line_end() {
        assertEquals(listOf("a", "", "b"), sourceLines("a\n\nb"))
        assertEquals(listOf("a", "b"), sourceLines("a\r\nb"))
        assertEquals(listOf(""), sourceLines("\n"))
    }

    @Test
    fun folder_of_a_path_is_its_parent_or_root() {
        assertEquals("src/auth", folderOf("src/auth/session.py"))
        assertEquals("src", folderOf("src/main.kt"))
        assertEquals("", folderOf("README.md"))
    }

    @Test
    fun folders_have_no_size_text_and_files_use_the_formatter() {
        val folder = FileEntry("src", "src", isDir = true, sizeBytes = 0L, ignored = false)
        val file = FileEntry("a.kt", "src/a.kt", isDir = false, sizeBytes = 2048L, ignored = false)
        var asked: Long? = null
        val format: (Long) -> String = {
            asked = it
            "$it B"
        }
        assertNull(browserSizeText(folder, format))
        assertNull(asked)
        assertEquals("2048 B", browserSizeText(file, format))
        assertEquals(2048L, asked)
    }

    @Test
    fun the_truncated_note_formats_the_full_file_size() {
        val preview =
            FilePreview(
                path = "big.log",
                sizeBytes = 300_000L,
                text = "",
                truncated = true,
                binary = false,
            )
        assertEquals("300000 B", truncatedSizeText(preview) { "$it B" })
    }
}
