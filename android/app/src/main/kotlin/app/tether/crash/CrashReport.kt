package app.tether.crash

import java.time.Instant

/** The longest message the report's header keeps. The stack trace keeps the full text. */
internal const val MESSAGE_LIMIT = 200

/**
 * The text saved for one crash. The first line names the exception and the time, so the About
 * card can show it alone. The stack trace includes every `Caused by` chain.
 */
internal fun formatCrashReport(
    now: Instant,
    versionName: String,
    threadName: String,
    error: Throwable,
): String =
    buildString {
        append(error.javaClass.name).append(" at ").append(now).append('\n')
        append("Version: ").append(versionName).append('\n')
        append("Thread: ").append(threadName).append('\n')
        append("Message: ").append(cutMessage(error.message)).append("\n\n")
        append(error.stackTraceToString())
    }

/**
 * The message cut to [limit] characters. A cut never leaves half of a surrogate pair, so the
 * text stays valid. A null message becomes an empty string.
 */
internal fun cutMessage(
    message: String?,
    limit: Int = MESSAGE_LIMIT,
): String {
    if (message == null || message.length <= limit) return message.orEmpty()
    val end = if (message[limit - 1].isHighSurrogate()) limit - 1 else limit
    return message.substring(0, end)
}
