package app.tether.preview

import org.junit.Assert.assertEquals
import org.junit.Test

class RequestDecisionTest {
    @Test
    fun the_preview_origin_is_served_from_the_project() {
        assertEquals(
            RequestDecision.Serve("index.html"),
            requestDecision("$PREVIEW_ORIGIN/", false),
        )
        assertEquals(
            RequestDecision.Serve("css/a.css"),
            requestDecision("$PREVIEW_ORIGIN/css/a.css", networkAllowed = true),
        )
    }

    @Test
    fun a_bad_path_on_the_preview_origin_is_not_found() {
        assertEquals(
            RequestDecision.NotFound,
            requestDecision("$PREVIEW_ORIGIN/../x", networkAllowed = false),
        )
        assertEquals(
            RequestDecision.NotFound,
            requestDecision("$PREVIEW_ORIGIN/..%2f..%2ffiles/databases/tether.db", true),
        )
        assertEquals(
            RequestDecision.NotFound,
            requestDecision("https://preview.tether.invalid:443/", true),
        )
    }

    @Test
    fun another_host_is_blocked_unless_the_network_is_on() {
        assertEquals(RequestDecision.Block, requestDecision("https://example.com/a.js", false))
        assertEquals(RequestDecision.Pass, requestDecision("https://example.com/a.js", true))
    }

    @Test
    fun user_info_cannot_hide_another_host() {
        val url = "https://preview.tether.invalid@evil.test/"
        assertEquals(RequestDecision.Block, requestDecision(url, false))
        assertEquals(RequestDecision.Pass, requestDecision(url, true))
    }

    @Test
    fun lookalike_hosts_are_other_hosts() {
        assertEquals(
            RequestDecision.Block,
            requestDecision("https://preview.tether.invalid.evil.test/", false),
        )
    }

    @Test
    fun http_is_blocked_even_with_the_network_on() {
        assertEquals(RequestDecision.Block, requestDecision("http://preview.tether.invalid/", true))
        assertEquals(RequestDecision.Block, requestDecision("http://example.com/", true))
        assertEquals(RequestDecision.Block, requestDecision("ftp://example.com/", true))
        assertEquals(RequestDecision.Block, requestDecision("file:///data/data/x", true))
        assertEquals(RequestDecision.Block, requestDecision("intent://example.com", true))
    }

    @Test
    fun data_and_blob_urls_pass() {
        assertEquals(RequestDecision.Pass, requestDecision("data:text/plain,hi", false))
        assertEquals(RequestDecision.Pass, requestDecision("blob:https://example.com/id", false))
    }

    @Test
    fun an_unparseable_url_is_blocked() {
        assertEquals(RequestDecision.Block, requestDecision("not a url", true))
    }
}
