package app.tether.files

import org.junit.Assert.assertEquals
import org.junit.Test

class FileTreeTest {
    private val src = FileEntry("src", "src", isDir = true, sizeBytes = 0L, ignored = false)
    private val readme =
        FileEntry("README.md", "README.md", isDir = false, sizeBytes = 10L, ignored = false)
    private val main =
        FileEntry("main.kt", "src/main.kt", isDir = false, sizeBytes = 5L, ignored = false)

    @Test
    fun before_the_root_loads_only_a_loading_row_shows() {
        assertEquals(
            listOf<TreeRow>(TreeRow.Loading(ROOT_DIR, 0)),
            visibleRows(emptyMap(), emptySet()),
        )
    }

    @Test
    fun a_collapsed_folder_hides_its_children_but_the_cache_keeps_them() {
        val cache =
            mapOf(
                ROOT_DIR to DirState.Loaded(listOf(src, readme), truncated = false),
                "src" to DirState.Loaded(listOf(main), truncated = false),
            )
        val rows = visibleRows(cache, expanded = emptySet())
        assertEquals(
            listOf<TreeRow>(
                TreeRow.Entry(src, depth = 0, expanded = false),
                TreeRow.Entry(readme, depth = 0, expanded = false),
            ),
            rows,
        )
    }

    @Test
    fun an_expanded_folder_shows_its_children_one_level_deeper() {
        val cache =
            mapOf(
                ROOT_DIR to DirState.Loaded(listOf(src, readme), truncated = false),
                "src" to DirState.Loaded(listOf(main), truncated = false),
            )
        val rows = visibleRows(cache, expanded = setOf("src"))
        assertEquals(
            listOf<TreeRow>(
                TreeRow.Entry(src, depth = 0, expanded = true),
                TreeRow.Entry(main, depth = 1, expanded = false),
                TreeRow.Entry(readme, depth = 0, expanded = false),
            ),
            rows,
        )
    }

    @Test
    fun an_expanded_folder_without_a_listing_shows_a_loading_row() {
        val cache = mapOf(ROOT_DIR to DirState.Loaded(listOf(src), truncated = false))
        val rows = visibleRows(cache, expanded = setOf("src"))
        assertEquals(
            listOf<TreeRow>(
                TreeRow.Entry(src, depth = 0, expanded = true),
                TreeRow.Loading("src", depth = 1),
            ),
            rows,
        )
    }

    @Test
    fun an_empty_folder_says_so_and_a_truncated_listing_ends_with_a_note() {
        val cache = mapOf(ROOT_DIR to DirState.Loaded(emptyList(), truncated = true))
        assertEquals(
            listOf<TreeRow>(TreeRow.Empty(0), TreeRow.Truncated(0)),
            visibleRows(cache, emptySet()),
        )
    }

    @Test
    fun a_failed_folder_shows_a_failed_row_at_its_depth() {
        val cache =
            mapOf(
                ROOT_DIR to DirState.Loaded(listOf(src), truncated = false),
                "src" to DirState.Failed,
            )
        assertEquals(
            listOf<TreeRow>(
                TreeRow.Entry(src, depth = 0, expanded = true),
                TreeRow.Failed("src", depth = 1),
            ),
            visibleRows(cache, expanded = setOf("src")),
        )
    }
}
