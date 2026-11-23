package app.tether.settings.mcp

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class McpValidationTest {
    @Test
    fun server_names_follow_the_core_rule() {
        val valid = listOf("a", "docs-search", "0-wiki", "a".repeat(MCP_NAME_MAX))
        valid.forEach { assertTrue("expected valid: $it", isValidServerName(it)) }

        val invalid =
            listOf(
                "",
                "-docs",
                "Docs",
                "docs_search",
                "docs search",
                "docs.search",
                "a".repeat(MCP_NAME_MAX + 1),
            )
        invalid.forEach { assertFalse("expected invalid: $it", isValidServerName(it)) }
    }

    @Test
    fun server_names_allow_exactly_40_characters() {
        assertEquals(40, MCP_NAME_MAX)
        assertTrue(isValidServerName("a".repeat(40)))
        assertFalse(isValidServerName("a".repeat(41)))
    }

    @Test
    fun urls_must_be_https_with_a_host_and_no_user_info() {
        val valid =
            listOf("https://mcp.example.com/docs", "https://host", "https://host:8443/x?y=1")
        valid.forEach { assertTrue("expected valid: $it", isValidServerUrl(it)) }

        val invalid =
            listOf(
                "",
                "http://mcp.example.com",
                "https://",
                "https:///docs",
                "https://user:secret@mcp.example.com",
                "https://user@mcp.example.com",
                "https://mcp example.com",
                "https://mcp.example.com/a b",
                "mcp.example.com",
            )
        invalid.forEach { assertFalse("expected invalid: $it", isValidServerUrl(it)) }
    }

    @Test
    fun hostOf_drops_scheme_path_query_and_fragment() {
        assertEquals("mcp.example.com", hostOf("https://mcp.example.com/docs?x=1#top"))
        assertEquals("h:8443", hostOf("https://h:8443"))
        assertEquals("", hostOf("https://"))
    }

    @Test
    fun header_names_are_letters_digits_and_hyphens_up_to_64() {
        assertTrue(isValidHeaderName("X-Team"))
        assertTrue(isValidHeaderName("a".repeat(MCP_HEADER_NAME_MAX)))
        assertFalse(isValidHeaderName("a".repeat(MCP_HEADER_NAME_MAX + 1)))
        assertFalse(isValidHeaderName(""))
        assertFalse(isValidHeaderName("X_Team"))
        assertFalse(isValidHeaderName("X Team"))
        assertFalse(isValidHeaderName("X:Team"))
    }

    @Test
    fun header_values_are_not_blank_and_at_most_4_KiB() {
        assertTrue(isValidHeaderValue("token-value"))
        assertTrue(isValidHeaderValue("a".repeat(MCP_HEADER_VALUE_MAX_BYTES)))
        assertFalse(isValidHeaderValue("a".repeat(MCP_HEADER_VALUE_MAX_BYTES + 1)))
        assertFalse(isValidHeaderValue(""))
        assertFalse(isValidHeaderValue("   "))
    }

    @Test
    fun header_value_size_counts_utf8_bytes() {
        // Each "e with acute" is two bytes in UTF-8.
        assertTrue(isValidHeaderValue("é".repeat(MCP_HEADER_VALUE_MAX_BYTES / 2)))
        assertFalse(isValidHeaderValue("é".repeat(MCP_HEADER_VALUE_MAX_BYTES / 2 + 1)))
    }

    @Test
    fun header_values_reject_line_breaks_and_control_characters() {
        assertFalse(isValidHeaderValue("abc\ndef"))
        assertFalse(isValidHeaderValue("abc\rdef"))
        assertFalse(isValidHeaderValue("abc\u0000"))
    }

    @Test
    fun the_form_allows_at_most_8_headers() {
        assertEquals(8, MCP_MAX_HEADERS)
        val full = AddMcpServerUiState(headers = List(MCP_MAX_HEADERS) { HeaderDraft() })
        assertFalse(full.canAddHeader)
        assertTrue(full.copy(headers = full.headers.drop(1)).canAddHeader)
    }

    @Test
    fun a_description_under_the_limit_is_unchanged() {
        assertEquals("Search the docs.", plainDescription("Search the docs."))
        val exact = "a".repeat(MCP_DESCRIPTION_MAX)
        assertEquals(exact, plainDescription(exact))
    }

    @Test
    fun a_long_description_is_cut_to_300_characters_with_an_ellipsis() {
        val cut = plainDescription("a".repeat(MCP_DESCRIPTION_MAX + 1))
        assertEquals(MCP_DESCRIPTION_MAX, cut.length)
        assertTrue(cut.endsWith("…"))
        assertEquals("a".repeat(MCP_DESCRIPTION_MAX - 1), cut.dropLast(1))
    }

    @Test
    fun description_control_characters_become_spaces_but_line_breaks_stay() {
        assertEquals("one two\nthree", plainDescription("one\ttwo\nthree"))
        assertEquals("a b", plainDescription("a\u0007b"))
    }

    @Test
    fun a_description_is_never_read_as_markup() {
        val text = "[click](https://evil.example) <b>bold</b> **x**"
        assertEquals(text, plainDescription(text))
    }
}
