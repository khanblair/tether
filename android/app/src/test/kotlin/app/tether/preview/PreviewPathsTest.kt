package app.tether.preview

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class PreviewPathsTest {
    @Test
    fun the_root_and_folders_serve_their_index_page() {
        assertEquals("index.html", previewPathFor("$PREVIEW_ORIGIN/"))
        assertEquals("index.html", previewPathFor(PREVIEW_ORIGIN))
        assertEquals("docs/index.html", previewPathFor("$PREVIEW_ORIGIN/docs/"))
        assertEquals("index.html", previewPathFor("$PREVIEW_ORIGIN/"))
    }

    @Test
    fun a_file_path_maps_to_itself() {
        assertEquals("index.html", previewPathFor("$PREVIEW_ORIGIN/index.html"))
        assertEquals("css/site.css", previewPathFor("$PREVIEW_ORIGIN/css/site.css"))
    }

    @Test
    fun the_query_and_fragment_are_dropped() {
        assertEquals("app.js", previewPathFor("$PREVIEW_ORIGIN/app.js?v=3#top"))
        assertEquals("index.html", previewPathFor("$PREVIEW_ORIGIN/?q=../../x"))
    }

    @Test
    fun the_path_is_decoded_once() {
        assertEquals("my notes.html", previewPathFor("$PREVIEW_ORIGIN/my%20notes.html"))
        assertEquals("a%2e.html", previewPathFor("$PREVIEW_ORIGIN/a%252e.html"))
    }

    @Test
    fun an_encoded_slash_or_dot_segment_is_still_refused() {
        assertNull(previewPathFor("$PREVIEW_ORIGIN/..%2f..%2ffiles/databases/tether.db"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/%2e%2e/x"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/%2e/x"))
    }

    @Test
    fun dot_dot_dot_and_empty_segments_are_refused() {
        assertNull(previewPathFor("$PREVIEW_ORIGIN/../x"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/a/../b"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/./x"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/a//b"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN//x"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/a//"))
    }

    @Test
    fun backslash_nul_and_bad_escapes_are_refused() {
        assertNull(previewPathFor("$PREVIEW_ORIGIN/a%5Cb"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/a%00b"))
        assertNull(previewPathFor("$PREVIEW_ORIGIN/a%zz"))
    }

    @Test
    fun other_schemes_hosts_ports_and_user_info_are_refused() {
        assertNull(previewPathFor("http://preview.tether.invalid/"))
        assertNull(previewPathFor("https://example.com/"))
        assertNull(previewPathFor("https://preview.tether.invalid.evil.test/"))
        assertNull(previewPathFor("https://preview.tether.invalid@evil.test/"))
        assertNull(previewPathFor("https://preview.tether.invalid:8443/"))
        assertNull(previewPathFor("https://preview.tether.invalid:443/"))
        assertNull(previewPathFor("//evil.test/x"))
        assertNull(previewPathFor("not a url"))
    }

    @Test
    fun the_host_name_is_case_insensitive() {
        assertEquals("index.html", previewPathFor("https://PREVIEW.tether.invalid/"))
    }

    @Test
    fun previewUrl_round_trips_through_previewPathFor() {
        for (path in listOf("index.html", "css/site.css", "my notes/a%b.html", "ñ/ü.js")) {
            assertEquals(path, previewPathFor(previewUrl(path)))
        }
        assertEquals("$PREVIEW_ORIGIN/my%20notes.html", previewUrl("my notes.html"))
    }

    @Test
    fun only_html_and_htm_are_previewable() {
        assertTrue(isPreviewable("index.html"))
        assertTrue(isPreviewable("docs/Page.HTM"))
        assertFalse(isPreviewable("style.css"))
        assertFalse(isPreviewable("html"))
        assertFalse(isPreviewable("notes.html.txt"))
    }
}
