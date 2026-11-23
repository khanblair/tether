package app.tether.settings.github

/** The wait between device-flow polls when GitHub asks for a slower rate. */
const val SLOW_DOWN_STEP_SECONDS = 5L

/** The shortest wait between polls, so a bad reply can never cause a tight loop. */
private const val MIN_INTERVAL_SECONDS = 1L

/** Milliseconds in one second. */
private const val MILLIS_PER_SECOND = 1000L

/**
 * The timing of a device-flow poll loop. Pure: the clock is injected, so a test advances time
 * without waiting. [delayMillis] is the wait before the next poll. [isExpired] is true once the
 * code's own expiry time has passed on the local clock.
 */
class DevicePollTiming(
    intervalSeconds: Long,
    private val expiresAtMillis: Long,
    private val clock: () -> Long,
) {
    private var intervalMillis: Long =
        intervalSeconds.coerceAtLeast(MIN_INTERVAL_SECONDS) * MILLIS_PER_SECOND

    /** The wait before the next poll, in milliseconds. */
    fun delayMillis(): Long = intervalMillis

    /** GitHub asked for a slower rate: wait [SLOW_DOWN_STEP_SECONDS] longer from now on. */
    fun slowDown() {
        intervalMillis += SLOW_DOWN_STEP_SECONDS * MILLIS_PER_SECOND
    }

    /** True once the local clock reaches the code's expiry time. */
    fun isExpired(): Boolean = clock() >= expiresAtMillis

    /** Whole seconds left until expiry, never below zero. */
    fun secondsLeft(): Long = secondsUntil(expiresAtMillis, clock())
}

/** Whole seconds from [nowMillis] to [expiresAtMillis], never below zero. */
fun secondsUntil(
    expiresAtMillis: Long,
    nowMillis: Long,
): Long = ((expiresAtMillis - nowMillis) / MILLIS_PER_SECOND).coerceAtLeast(0L)
