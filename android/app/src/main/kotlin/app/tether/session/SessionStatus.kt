package app.tether.session

import java.util.Locale

private const val PERCENT = 100L
private const val MICROS_PER_DOLLAR = 1_000_000.0

/** Saved usage and the selected model. A missing catalog context stays unknown. */
data class SessionStatus(
    val modelId: String,
    val mode: String,
    val tokensIn: Long,
    val tokensOut: Long,
    val costMicros: Long,
    val contextWindow: Long?,
) {
    val contextPercent: Int?
        get() =
            contextWindow?.takeIf { it > 0 }?.let { window ->
                ((tokensIn * PERCENT) / window).coerceIn(0, PERCENT).toInt()
            }

    fun costText(): String = String.format(Locale.US, "%.2f", costMicros / MICROS_PER_DOLLAR)
}

/**
 * The status line numbers from what the core already reported for the snapshot. Do not read
 * `tether.db` with Android's own SQLite here, or anywhere else in the app. A second SQLite library
 * on the same write-ahead-log database, in the same process, resets the shared WAL index that the
 * Rust core has mapped, and the core then dies with SIGBUS on its next read.
 */
internal fun statusFrom(
    details: SessionDetails,
    usage: SessionUsage,
    contextWindow: Long?,
): SessionStatus =
    SessionStatus(
        modelId = details.modelId,
        mode = details.mode.statusName(),
        tokensIn = usage.tokensIn,
        tokensOut = usage.tokensOut,
        costMicros = usage.costMicros,
        contextWindow = contextWindow,
    )

/** The mode name the status line reads: `accept_edits`, `ask`, `bypass`, and so on. */
internal fun SessionMode.statusName(): String =
    when (this) {
        SessionMode.ACCEPT_EDITS -> "accept_edits"
        else -> name.lowercase()
    }
