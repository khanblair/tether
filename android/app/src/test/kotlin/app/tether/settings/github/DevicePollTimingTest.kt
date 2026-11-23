package app.tether.settings.github

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class DevicePollTimingTest {
    private var now = 0L

    private fun timing(
        intervalSeconds: Long,
        expiresInSeconds: Long,
    ) = DevicePollTiming(intervalSeconds, expiresAtMillis = expiresInSeconds * 1000) { now }

    @Test
    fun the_first_delay_is_the_grant_interval() {
        assertEquals(5_000L, timing(5, 900).delayMillis())
    }

    @Test
    fun slow_down_adds_five_seconds_to_every_later_wait() {
        val timing = timing(5, 900)
        timing.slowDown()
        assertEquals(10_000L, timing.delayMillis())
        timing.slowDown()
        assertEquals(15_000L, timing.delayMillis())
    }

    @Test
    fun a_zero_interval_is_raised_to_one_second() {
        assertEquals(1_000L, timing(0, 900).delayMillis())
    }

    @Test
    fun expiry_is_reached_at_the_expiry_second_not_before() {
        val timing = timing(5, 12)
        now = 11_999
        assertFalse(timing.isExpired())
        now = 12_000
        assertTrue(timing.isExpired())
    }

    @Test
    fun seconds_left_count_down_and_stop_at_zero() {
        val timing = timing(5, 10)
        assertEquals(10L, timing.secondsLeft())
        now = 9_500
        assertEquals(0L, timing.secondsLeft())
        now = 20_000
        assertEquals(0L, timing.secondsLeft())
    }

    @Test
    fun seconds_until_never_goes_negative() {
        assertEquals(3L, secondsUntil(expiresAtMillis = 3_000, nowMillis = 0))
        assertEquals(0L, secondsUntil(expiresAtMillis = 3_000, nowMillis = 4_000))
    }
}
