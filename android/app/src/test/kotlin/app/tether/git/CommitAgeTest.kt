package app.tether.git

import org.junit.Assert.assertEquals
import org.junit.Test

class CommitAgeTest {
    private val now = 1_700_000_000_000L

    private fun secondsAgo(seconds: Long): Long = (now - seconds * 1_000L) / 1_000L

    @Test
    fun under_a_minute_is_just_now() {
        assertEquals(CommitAge.JustNow, commitAge(now, secondsAgo(59)))
    }

    @Test
    fun a_commit_in_the_future_is_just_now() {
        assertEquals(CommitAge.JustNow, commitAge(now, secondsAgo(-600)))
    }

    @Test
    fun one_minute_to_fifty_nine_minutes_counts_minutes() {
        assertEquals(CommitAge.Minutes(1), commitAge(now, secondsAgo(60)))
        assertEquals(CommitAge.Minutes(59), commitAge(now, secondsAgo(59 * 60 + 59)))
    }

    @Test
    fun one_hour_to_twenty_three_hours_counts_hours() {
        assertEquals(CommitAge.Hours(3), commitAge(now, secondsAgo(3 * 3_600 + 5)))
        assertEquals(CommitAge.Hours(23), commitAge(now, secondsAgo(23 * 3_600 + 1)))
    }

    @Test
    fun between_one_and_two_days_is_yesterday() {
        assertEquals(CommitAge.Yesterday, commitAge(now, secondsAgo(24 * 3_600)))
        assertEquals(CommitAge.Yesterday, commitAge(now, secondsAgo(47 * 3_600)))
    }

    @Test
    fun two_days_or_more_is_a_date_of_that_commit() {
        val age = commitAge(now, secondsAgo(3 * 24 * 3_600))

        assertEquals(CommitAge.OnDate((now - 3 * 24 * 3_600 * 1_000L)), age)
    }
}
