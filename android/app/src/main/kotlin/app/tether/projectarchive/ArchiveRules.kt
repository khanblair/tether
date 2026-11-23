package app.tether.projectarchive

/** The largest ZIP the app copies for an import: 200 MiB. The core has its own limits too. */
const val ARCHIVE_IMPORT_LIMIT_BYTES: Long = 200L * 1024L * 1024L

private const val BYTES_PER_KB = 1024L
private const val BYTES_PER_MB = 1024L * 1024L
private const val TENTHS = 10L

/** Whether a picked ZIP is too large to import. An unknown size is not refused here. */
fun isOverImportLimit(sizeBytes: Long?): Boolean =
    sizeBytes != null && sizeBytes > ARCHIVE_IMPORT_LIMIT_BYTES

/** The unit of an [ArchiveSize]. */
enum class ArchiveSizeUnit {
    /** Kilobytes, for sizes under 1 MB. */
    Kilobytes,

    /** Megabytes with one decimal, for sizes of 1 MB and up. */
    Megabytes,
}

/** A size split into its number, as text, and its unit, for a string resource. */
data class ArchiveSize(
    val amount: String,
    val unit: ArchiveSizeUnit,
)

/**
 * Formats a byte count. Under 1 MB it becomes whole kilobytes, rounded up and at least 1. From
 * 1 MB up it becomes megabytes, rounded to one decimal. Negative counts count as zero.
 */
fun formatArchiveSize(bytes: Long): ArchiveSize {
    val safe = bytes.coerceAtLeast(0L)
    if (safe < BYTES_PER_MB) {
        val kilobytes = ((safe + BYTES_PER_KB - 1) / BYTES_PER_KB).coerceAtLeast(1L)
        return ArchiveSize(kilobytes.toString(), ArchiveSizeUnit.Kilobytes)
    }
    val whole = safe / BYTES_PER_MB
    val remainder = safe % BYTES_PER_MB
    val tenths = whole * TENTHS + (remainder * TENTHS + BYTES_PER_MB / 2) / BYTES_PER_MB
    return ArchiveSize("${tenths / TENTHS}.${tenths % TENTHS}", ArchiveSizeUnit.Megabytes)
}

/**
 * The project name suggested from a picked file name: the name without its last extension. Returns
 * an empty string when there is no name.
 */
fun importNameFromFile(displayName: String?): String =
    displayName?.substringBeforeLast('.').orEmpty()

/**
 * The file name suggested for an export: the project name with characters that file pickers reject
 * replaced by a dash, plus `.zip`.
 */
fun exportFileName(projectName: String): String {
    val safe = projectName.replace(Regex("[/\\\\:*?\"<>|]"), "-").trim()
    return (safe.ifEmpty { "project" }) + ".zip"
}
