package app.tether.git

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

private const val MINUTE_MS = 60_000L
private const val HOUR_MS = 60 * MINUTE_MS
private const val DAY_MS = 24 * HOUR_MS
private const val MILLIS_PER_SECOND = 1_000L

/** How long ago a commit was made, in the buckets the History tab shows. */
sealed interface CommitAge {
    /** Less than a minute ago, or a time in the future. */
    data object JustNow : CommitAge

    /** One to 59 minutes ago. */
    data class Minutes(
        val count: Int,
    ) : CommitAge

    /** One to 23 hours ago. */
    data class Hours(
        val count: Int,
    ) : CommitAge

    /** Between 24 and 48 hours ago. */
    data object Yesterday : CommitAge

    /** Older than two days. Shown as a date in the device time zone. */
    data class OnDate(
        val epochMillis: Long,
    ) : CommitAge
}

/**
 * Buckets the time from a commit at [committedSeconds] (Unix seconds) until [nowMillis]. A commit
 * in the future counts as [CommitAge.JustNow].
 */
fun commitAge(
    nowMillis: Long,
    committedSeconds: Long,
): CommitAge {
    val committedMillis = committedSeconds * MILLIS_PER_SECOND
    val elapsed = nowMillis - committedMillis
    return when {
        elapsed < MINUTE_MS -> CommitAge.JustNow
        elapsed < HOUR_MS -> CommitAge.Minutes((elapsed / MINUTE_MS).toInt())
        elapsed < DAY_MS -> CommitAge.Hours((elapsed / HOUR_MS).toInt())
        elapsed < 2 * DAY_MS -> CommitAge.Yesterday
        else -> CommitAge.OnDate(committedMillis)
    }
}

/** The words for this bucket, such as "3 h ago", or the date for an older commit. */
@Composable
fun CommitAge.text(): String =
    when (this) {
        CommitAge.JustNow -> stringResource(R.string.git_time_just_now)
        is CommitAge.Minutes -> stringResource(R.string.git_time_minutes_ago, count)
        is CommitAge.Hours -> stringResource(R.string.git_time_hours_ago, count)
        CommitAge.Yesterday -> stringResource(R.string.git_time_yesterday)
        is CommitAge.OnDate -> dateText(epochMillis)
    }

private fun dateText(epochMillis: Long): String =
    DateTimeFormatter
        .ofLocalizedDate(FormatStyle.MEDIUM)
        .withZone(ZoneId.systemDefault())
        .format(Instant.ofEpochMilli(epochMillis))
