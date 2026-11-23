package app.tether.settings.executors

import java.net.URI
import java.time.Instant
import java.time.format.DateTimeParseException

private const val BYTES_PER_MB = 1_000_000L
private const val SECONDS_PER_HOUR = 3_600L
private const val SECONDS_PER_MINUTE = 60L

/** Whole megabytes for [bytes], rounded to the nearest. Any size above zero shows at least 1 MB. */
fun megabytes(bytes: Long): Long =
    when {
        bytes <= 0L -> 0L
        else -> maxOf(1L, (bytes + BYTES_PER_MB / 2) / BYTES_PER_MB)
    }

/** The fraction of a download that is done, from 0 to 1. Unknown totals show 0. */
fun downloadFraction(
    downloaded: Long,
    total: Long,
): Float =
    if (total <= 0L) {
        0f
    } else {
        (downloaded.toFloat() / total.toFloat()).coerceIn(0f, 1f)
    }

/** A duration split into hours, minutes, and seconds. */
data class DurationParts(
    val hours: Long,
    val minutes: Long,
    val seconds: Long,
)

/** Splits [totalSeconds] into parts. A negative value counts as zero. */
fun durationParts(totalSeconds: Long): DurationParts {
    val safe = maxOf(0L, totalSeconds)
    return DurationParts(
        hours = safe / SECONDS_PER_HOUR,
        minutes = (safe % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE,
        seconds = safe % SECONDS_PER_MINUTE,
    )
}

/** The host name of a manifest URL, so the screen can show where a download comes from. */
fun manifestHost(url: String?): String? = url?.let { runCatching { URI(it).host }.getOrNull() }

/** Epoch milliseconds for a run's start time, or null when the text is not an ISO instant. */
fun parseStartedAt(text: String): Long? =
    try {
        Instant.parse(text).toEpochMilli()
    } catch (error: DateTimeParseException) {
        null
    }

/** The step of a Linux setup, as far as the core's byte progress can tell. */
enum class LinuxPhase {
    /** No progress has arrived yet. */
    Starting,

    /** Bytes are arriving, or the size is not known yet. */
    Downloading,

    /** Every byte has arrived. The core is checking and unpacking the files. */
    SettingUp,
}

/** The setup step for the byte counts seen so far. */
fun linuxPhaseOf(
    downloaded: Long,
    total: Long,
): LinuxPhase =
    when {
        total > 0L && downloaded >= total -> LinuxPhase.SettingUp
        downloaded > 0L || total > 0L -> LinuxPhase.Downloading
        else -> LinuxPhase.Starting
    }
