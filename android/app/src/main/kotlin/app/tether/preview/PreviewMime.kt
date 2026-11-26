package app.tether.preview

/** The content type a preview response carries. [encoding] is "utf-8" for text, null for bytes. */
data class PreviewMime(
    val mimeType: String,
    val encoding: String?,
)

private const val UTF_8 = "utf-8"
private const val OCTET_STREAM = "application/octet-stream"

private val TEXT_TYPES =
    mapOf(
        "html" to "text/html",
        "htm" to "text/html",
        "css" to "text/css",
        "js" to "text/javascript",
        "mjs" to "text/javascript",
        "json" to "application/json",
        "map" to "application/json",
        "svg" to "image/svg+xml",
        "xml" to "application/xml",
        "txt" to "text/plain",
        "md" to "text/markdown",
    )

private val BYTE_TYPES =
    mapOf(
        "png" to "image/png",
        "jpg" to "image/jpeg",
        "jpeg" to "image/jpeg",
        "gif" to "image/gif",
        "webp" to "image/webp",
        "ico" to "image/x-icon",
        "woff" to "font/woff",
        "woff2" to "font/woff2",
        "ttf" to "font/ttf",
        "otf" to "font/otf",
        "wasm" to "application/wasm",
        "mp3" to "audio/mpeg",
        "mp4" to "video/mp4",
        "webm" to "video/webm",
    )

/** The content type for the file at [path], from its extension. Unknown types are octet streams. */
fun mimeFor(path: String): PreviewMime {
    val extension = path.substringAfterLast('/').substringAfterLast('.', "").lowercase()
    return when {
        TEXT_TYPES.containsKey(extension) -> PreviewMime(TEXT_TYPES.getValue(extension), UTF_8)
        BYTE_TYPES.containsKey(extension) -> PreviewMime(BYTE_TYPES.getValue(extension), null)
        else -> PreviewMime(OCTET_STREAM, null)
    }
}
