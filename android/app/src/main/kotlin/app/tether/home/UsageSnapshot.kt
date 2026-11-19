package app.tether.home

import androidx.compose.runtime.Immutable

/** The days in the usage chart: the last seven, today included. */
const val WEEK_DAYS = 7

/** The totals of one day: sessions run, tokens used, and files changed. */
@Immutable
data class DayTotals(
    val sessions: Int,
    val tokens: Long,
    val filesChanged: Int,
)

/** Tokens used by one model. */
@Immutable
data class ModelUsage(
    val model: String,
    val tokens: Long,
)

/**
 * The usage the Home cards show. [lastSevenDays] holds the tokens per day, oldest day first, so
 * the last entry is today.
 */
@Immutable
data class UsageSnapshot(
    val today: DayTotals,
    val yesterday: DayTotals,
    val lastSevenDays: List<Long>,
    val previousWeekTokens: Long,
    val byModel: List<ModelUsage>,
) {
    init {
        require(lastSevenDays.size == WEEK_DAYS) { "lastSevenDays needs seven days" }
    }

    /** The tokens used in the last seven days. */
    val weekTokens: Long = lastSevenDays.sum()

    /** True when any usage shows: this week, last week, or a session today. */
    val hasUsage: Boolean
        get() = weekTokens > 0L || previousWeekTokens > 0L || today.sessions > 0

    companion object {
        /** No usage at all. Release builds show this until the core reports real totals. */
        val Empty =
            UsageSnapshot(
                today = DayTotals(sessions = 0, tokens = 0L, filesChanged = 0),
                yesterday = DayTotals(sessions = 0, tokens = 0L, filesChanged = 0),
                lastSevenDays = List(WEEK_DAYS) { 0L },
                previousWeekTokens = 0L,
                byModel = emptyList(),
            )
    }
}
