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
private const val QUERY_SAFE_PUNCTUATION = "-._~"
private const val PERCENT = '%'
private const val HEX_RADIX = 16
private const val ESCAPE_LENGTH = 3
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
internal fun encodePath(path: String): String = percentEncode(path, SAFE_PUNCTUATION)

/**
 * Percent-encodes one query value byte by byte, keeping letters, digits and "-._~". Every other
 * byte, including "/", ":", "?", "&", "=" and "#", is encoded, so the value cannot split the URL.
 */
internal fun encodeQueryValue(value: String): String = percentEncode(value, QUERY_SAFE_PUNCTUATION)

/**
 * Reverses [encodeQueryValue]: each %XX escape is one UTF-8 byte. Null when an escape is cut
 * short or is not two hex digits. Pure Kotlin, so it runs in plain JVM tests.
 */
internal fun decodeQueryValue(value: String): String? {
    val bytes = java.io.ByteArrayOutputStream()
    var i = 0
    var valid = true
    while (valid && i < value.length) {
        if (value[i] == PERCENT) {
            val byte = escapedByte(value, i)
            valid = byte != null
            if (byte != null) bytes.write(byte)
            i += ESCAPE_LENGTH
        } else {
            val codePoint = value.codePointAt(i)
            bytes.write(String(Character.toChars(codePoint)).toByteArray(Charsets.UTF_8))
            i += Character.charCount(codePoint)
        }
    }
    return if (valid) String(bytes.toByteArray(), Charsets.UTF_8) else null
}

/** The byte of the %XX escape that starts at [index], or null when it is cut short or not hex. */
private fun escapedByte(
    value: String,
    index: Int,
): Int? {
    val high = value.getOrNull(index + 1)?.let { Character.digit(it, HEX_RADIX) } ?: -1
    val low = value.getOrNull(index + 2)?.let { Character.digit(it, HEX_RADIX) } ?: -1
    return if (high < 0 || low < 0) null else high * HEX_RADIX + low
}

/** Percent-encodes [text] byte by byte, keeping ASCII letters, digits and [safePunctuation]. */
private fun percentEncode(
    text: String,
    safePunctuation: String,
): String {
    val out = StringBuilder()
    for (byte in text.toByteArray(Charsets.UTF_8)) {
        val code = byte.toInt() and BYTE_MASK
        val char = code.toChar()
        val safe =
            code < ASCII_LIMIT && (char.isLetterOrDigit() || safePunctuation.contains(char))
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
