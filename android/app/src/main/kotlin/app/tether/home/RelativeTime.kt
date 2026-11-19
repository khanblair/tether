package app.tether.home

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R

private const val MINUTE_MS = 60_000L
private const val HOUR_MS = 60 * MINUTE_MS
private const val DAY_MS = 24 * HOUR_MS

/** How long ago something happened, in the buckets the screens show. */
sealed interface RelativeTime {
    /** Less than a minute ago, or a time in the future. */
    data object JustNow : RelativeTime

    /** One to 59 minutes ago. */
    data class Minutes(
        val n: Int,
    ) : RelativeTime

    /** One to 23 hours ago. */
    data class Hours(
        val n: Int,
    ) : RelativeTime

    /** Between 24 and 48 hours ago. */
    data object Yesterday : RelativeTime

    /** Two or more days ago. */
    data class Days(
        val n: Int,
    ) : RelativeTime
}

/**
 * Buckets the time between [then] and [now]. A future [then] counts as [RelativeTime.JustNow].
 */
fun relativeTime(
    now: Long,
    then: Long,
): RelativeTime {
    val elapsed = now - then
    return when {
        elapsed < MINUTE_MS -> RelativeTime.JustNow
        elapsed < HOUR_MS -> RelativeTime.Minutes((elapsed / MINUTE_MS).toInt())
        elapsed < DAY_MS -> RelativeTime.Hours((elapsed / HOUR_MS).toInt())
        elapsed < 2 * DAY_MS -> RelativeTime.Yesterday
        else -> RelativeTime.Days((elapsed / DAY_MS).toInt())
    }
}

/** The words for this bucket, such as "12 min ago". */
@Composable
fun RelativeTime.text(): String =
    when (this) {
        RelativeTime.JustNow -> stringResource(R.string.time_just_now)
        is RelativeTime.Minutes -> stringResource(R.string.time_minutes_ago, n)
        is RelativeTime.Hours -> stringResource(R.string.time_hours_ago, n)
        RelativeTime.Yesterday -> stringResource(R.string.time_yesterday)
        is RelativeTime.Days -> pluralStringResource(R.plurals.time_days_ago, n, n)
    }
