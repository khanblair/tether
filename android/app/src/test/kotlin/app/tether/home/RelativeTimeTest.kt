package app.tether.home

import org.junit.Assert.assertEquals
import org.junit.Test

class RelativeTimeTest {
    private val now = 10_000_000_000L

    private fun ago(elapsedMs: Long): RelativeTime = relativeTime(now = now, then = now - elapsedMs)

    private val second = 1_000L
    private val minute = 60 * second
    private val hour = 60 * minute
    private val day = 24 * hour

    @Test
    fun a_time_in_the_future_is_just_now() {
        assertEquals(RelativeTime.JustNow, relativeTime(now = now, then = now + 5 * second))
    }

    @Test
    fun fifty_nine_seconds_is_just_now() {
        assertEquals(RelativeTime.JustNow, ago(59 * second))
    }

    @Test
    fun sixty_seconds_is_one_minute() {
        assertEquals(RelativeTime.Minutes(1), ago(60 * second))
    }

    @Test
    fun fifty_nine_minutes_is_fifty_nine_minutes() {
        assertEquals(RelativeTime.Minutes(59), ago(59 * minute))
    }

    @Test
    fun sixty_minutes_is_one_hour() {
        assertEquals(RelativeTime.Hours(1), ago(60 * minute))
    }

    @Test
    fun twenty_three_hours_and_fifty_nine_minutes_is_twenty_three_hours() {
        assertEquals(RelativeTime.Hours(23), ago(23 * hour + 59 * minute))
    }

    @Test
    fun twenty_four_hours_is_yesterday() {
        assertEquals(RelativeTime.Yesterday, ago(day))
    }

    @Test
    fun forty_seven_hours_and_fifty_nine_minutes_is_still_yesterday() {
        assertEquals(RelativeTime.Yesterday, ago(47 * hour + 59 * minute))
    }

    @Test
    fun forty_eight_hours_is_two_days() {
        assertEquals(RelativeTime.Days(2), ago(2 * day))
    }

    @Test
    fun three_days_is_three_days() {
        assertEquals(RelativeTime.Days(3), ago(3 * day))
    }
}
