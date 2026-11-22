package app.tether.files

/**
 * The lines of [text], with `\r\n` and `\n` both as line ends. A trailing line end does not start
 * an extra empty line, and empty text has no lines.
 */
fun sourceLines(text: String): List<String> {
    if (text.isEmpty()) return emptyList()
    val lines = text.replace("\r\n", "\n").split("\n")
    return if (text.endsWith("\n") || text.endsWith("\r\n")) lines.dropLast(1) else lines
}

/** The folder part of a project path, or the empty string for a file at the root. */
fun folderOf(path: String): String = path.substringBeforeLast('/', missingDelimiterValue = "")

/**
 * The size text for a tree row. Folders have no size text, so the result is null for them.
 * [format] turns a byte count into words, such as "4.2 KB".
 */
fun browserSizeText(
    entry: FileEntry,
    format: (Long) -> String,
): String? = if (entry.isDir) null else format(entry.sizeBytes)

/** The size text in the note that says a file was cut short, such as "128 KB". */
fun truncatedSizeText(
    preview: FilePreview,
    format: (Long) -> String,
): String = format(preview.sizeBytes)
