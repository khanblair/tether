package app.tether.settings.mcp

/** Longest server name, in characters. */
const val MCP_NAME_MAX = 40

/** Longest header name, in characters. */
const val MCP_HEADER_NAME_MAX = 64

/** Largest header value, in UTF-8 bytes (4 KiB). */
const val MCP_HEADER_VALUE_MAX_BYTES = 4096

/** Most headers one server can carry. */
const val MCP_MAX_HEADERS = 8

/** Longest tool description shown, in characters, counting the ellipsis. */
const val MCP_DESCRIPTION_MAX = 300

private const val HTTPS_PREFIX = "https://"
private const val ELLIPSIS = "…"

private val NAME_RULE = Regex("[a-z0-9][a-z0-9-]{0,${MCP_NAME_MAX - 1}}")
private val HEADER_NAME_RULE = Regex("[A-Za-z0-9-]{1,$MCP_HEADER_NAME_MAX}")

/**
 * True when [name] fits the core's rule: a lowercase letter or digit first, then lowercase letters,
 * digits, or hyphens, up to 40 characters.
 */
fun isValidServerName(name: String): Boolean = NAME_RULE.matches(name)

/** True when [name] is a header name: letters, digits, or hyphens, up to 64 characters. */
fun isValidHeaderName(name: String): Boolean = HEADER_NAME_RULE.matches(name)

/**
 * True when [value] can be sent as a header: not blank, at most 4 KiB in UTF-8, and no control
 * characters, so no line break can split the header.
 */
fun isValidHeaderValue(value: String): Boolean =
    value.isNotBlank() &&
        value.none { it.isISOControl() } &&
        value.toByteArray(Charsets.UTF_8).size <= MCP_HEADER_VALUE_MAX_BYTES

/**
 * True when [url] starts with https://, has a host, and carries no user name or password. Only
 * ASCII printable characters are allowed, so no space or control character gets in.
 */
fun isValidServerUrl(url: String): Boolean {
    val printable = url.all { it.code > ' '.code && it.code <= '~'.code }
    val authority = hostOf(url)
    return url.startsWith(HTTPS_PREFIX) && printable && authority.isNotEmpty() && '@' !in authority
}

/** The host part of [url], without the scheme, path, query, or fragment. Empty if there is none. */
fun hostOf(url: String): String =
    url
        .removePrefix(HTTPS_PREFIX)
        .takeWhile { it != '/' && it != '?' && it != '#' }

/**
 * A tool description as plain text. Control characters other than a line break become spaces,
 * and text longer than [MCP_DESCRIPTION_MAX] is cut to that many characters, the last one an
 * ellipsis. A server controls this text, so nothing in it is ever read as markup.
 */
fun plainDescription(text: String): String {
    val clean = text.map { if (it.isISOControl() && it != '\n') ' ' else it }.joinToString("")
    if (clean.length <= MCP_DESCRIPTION_MAX) return clean
    return clean.take(MCP_DESCRIPTION_MAX - ELLIPSIS.length).trimEnd() + ELLIPSIS
}
