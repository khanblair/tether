package app.tether.preview

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PreviewMimeTest {
    @Test
    fun text_types_carry_utf_8() {
        assertEquals(PreviewMime("text/html", "utf-8"), mimeFor("index.html"))
        assertEquals(PreviewMime("text/html", "utf-8"), mimeFor("a/B.HTM"))
        assertEquals(PreviewMime("text/css", "utf-8"), mimeFor("site.css"))
        assertEquals(PreviewMime("text/javascript", "utf-8"), mimeFor("app.js"))
        assertEquals(PreviewMime("text/javascript", "utf-8"), mimeFor("app.mjs"))
        assertEquals(PreviewMime("application/json", "utf-8"), mimeFor("data.json"))
        assertEquals(PreviewMime("image/svg+xml", "utf-8"), mimeFor("logo.svg"))
        assertEquals(PreviewMime("text/plain", "utf-8"), mimeFor("notes.txt"))
        assertEquals(PreviewMime("text/markdown", "utf-8"), mimeFor("README.md"))
        assertEquals(PreviewMime("application/xml", "utf-8"), mimeFor("feed.xml"))
    }

    @Test
    fun byte_types_have_no_encoding() {
        assertEquals(PreviewMime("image/png", null), mimeFor("a.png"))
        assertEquals(PreviewMime("image/jpeg", null), mimeFor("a.JPG"))
        assertEquals(PreviewMime("image/jpeg", null), mimeFor("a.jpeg"))
        assertEquals(PreviewMime("font/woff2", null), mimeFor("f.woff2"))
        assertEquals(PreviewMime("application/wasm", null), mimeFor("m.wasm"))
        assertEquals(PreviewMime("video/webm", null), mimeFor("v.webm"))
    }

    @Test
    fun unknown_and_extensionless_files_are_octet_streams() {
        assertEquals(PreviewMime("application/octet-stream", null), mimeFor("tether.db"))
        assertEquals(PreviewMime("application/octet-stream", null), mimeFor("Makefile"))
        assertEquals(PreviewMime("application/octet-stream", null), mimeFor("dir.v1/file"))
        assertNull(mimeFor(".hidden").encoding)
    }
}
