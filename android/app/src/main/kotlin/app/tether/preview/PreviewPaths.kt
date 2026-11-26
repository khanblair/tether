package app.tether.preview

import java.net.URI
import java.net.URISyntaxException

/** The synthetic origin every preview page is served from. No real host has this name. */
const val PREVIEW_ORIGIN = "https://preview.tether.invalid"

/** The host of [PREVIEW_ORIGIN], compared without case. */
const val PREVIEW_HOST = "preview.tether.invalid"

/** The page a folder or the root serves. */
const val INDEX_FILE = "index.html"

private const val DOT = "."
private const val DOT_DOT = ".."
private const val BACKSLASH = '\\'
private const val NUL = '\u0000'
private const val ASCII_LIMIT = 128
private const val SAFE_PUNCTUATION = "-._~/"
private const val HEX_DIGITS = "0123456789ABCDEF"
private const val NIBBLE_BITS = 4
private const val NIBBLE_MASK = 0xF
private const val BYTE_MASK = 0xFF

/**
 * The preview URL of [path] in the project, such as "css/site.css". Each byte outside the safe
 * set is percent-encoded.
 */
fun previewUrl(path: String): String = "$PREVIEW_ORIGIN/" + encodePath(path)

/**
 * The project path that a request URL on the preview origin names, or null when the URL is not
 * on that origin or its path is unsafe. The query and fragment are dropped. The path is decoded
 * once, so an encoded slash or dot counts as the character it encodes. A trailing slash names a
 * folder and gives its index page. The root gives [INDEX_FILE].
 */
fun previewPathFor(url: String): String? {
    val uri = parseUri(url)?.takeIf(::isPreviewOrigin) ?: return null
    val decoded = uri.path.orEmpty().ifEmpty { "/" }
    return segmentsOf(decoded)?.joinToString("/")
}

/**
 * The segments of a decoded path, with the index page for a folder. Null when the path is unsafe.
 */
private fun segmentsOf(decoded: String): List<String>? {
    val folder = decoded.endsWith("/")
    val parts = decoded.removePrefix("/").split("/").let { if (folder) it.dropLast(1) else it }
    val names = if (folder || parts.isEmpty()) parts + INDEX_FILE else parts
    val safe =
        decoded.startsWith("/") &&
            decoded.none { it == BACKSLASH || it == NUL } &&
            parts.none { it.isEmpty() || it == DOT || it == DOT_DOT }
    return names.takeIf { safe }
}

/** Whether [path] is a page the preview can open: an `.html` or `.htm` file, in any case. */
fun isPreviewable(path: String): Boolean {
    val extension = path.substringAfterLast('/').substringAfterLast('.', "").lowercase()
    return extension == "html" || extension == "htm"
}

/** The URI of [url], or null when it is not a valid URI. */
internal fun parseUri(url: String): URI? =
    try {
        URI(url)
    } catch (_: URISyntaxException) {
        null
    }

/**
 * Whether [uri] names the preview host with no user info and no port. A host check alone is not
 * enough: "https://preview.tether.invalid@evil.test/" has the preview name as user info.
 */
internal fun isPreviewOrigin(uri: URI): Boolean =
    uri.scheme.equals("https", ignoreCase = true) &&
        uri.host.equals(PREVIEW_HOST, ignoreCase = true) &&
        uri.userInfo == null &&
        uri.port == -1

/** Percent-encodes [path] byte by byte, keeping letters, digits, "-._~" and "/". */
internal fun encodePath(path: String): String {
    val out = StringBuilder()
    for (byte in path.toByteArray(Charsets.UTF_8)) {
        val code = byte.toInt() and BYTE_MASK
        val char = code.toChar()
        val safe =
            code < ASCII_LIMIT && (char.isLetterOrDigit() || SAFE_PUNCTUATION.contains(char))
        if (safe) {
            out.append(char)
        } else {
            out
                .append('%')
                .append(HEX_DIGITS[code shr NIBBLE_BITS])
                .append(HEX_DIGITS[code and NIBBLE_MASK])
        }
    }
    return out.toString()
}
