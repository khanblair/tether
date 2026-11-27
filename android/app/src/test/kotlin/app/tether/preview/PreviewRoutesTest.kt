package app.tether.preview

import app.tether.nav.previewServerPath
import org.junit.Assert.assertEquals
import org.junit.Test

class PreviewRoutesTest {
    @Test
    fun server_route_encodes_url_and_process_id() {
        assertEquals(
            "preview/server?url=http%3A%2F%2F127.0.0.1%3A5173%2Findex.html&process=dev%2Fone",
            previewServerPath("http://127.0.0.1:5173/index.html", "dev/one"),
        )
    }

    @Test
    fun server_route_query_value_round_trips_through_pure_decoder() {
        val url = "http://127.0.0.1:5173/a?b=c"
        val route = previewServerPath(url, "dev")
        val encoded = route.substringAfter("url=").substringBefore("&process=")
        assertEquals(url, decodeQueryValue(encoded))
    }

    @Test
    fun decoder_rejects_cut_or_non_hex_escapes() {
        assertEquals(null, decodeQueryValue("abc%2"))
        assertEquals("a b", decodeQueryValue("a%20b"))
        assertEquals(null, decodeQueryValue("%zz"))
    }
}
