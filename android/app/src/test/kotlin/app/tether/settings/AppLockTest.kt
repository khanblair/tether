package app.tether.settings

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AppLockTest {
    private var now = 0L
    private var enabled = true

    private fun lock() = AppLock(isEnabled = { enabled }, clock = { now })

    @Test
    fun an_enabled_lock_starts_locked() {
        assertTrue(lock().locked.value)
    }

    @Test
    fun a_disabled_lock_starts_unlocked() {
        enabled = false

        assertFalse(lock().locked.value)
    }

    @Test
    fun time_away_below_the_limit_does_not_lock() {
        val appLock = lock().apply { endPrompt(unlocked = true) }
        now = 1_000L
        appLock.onStopped()
        now = 1_000L + AppLock.LOCK_AFTER_MS - 1

        appLock.onStarted()

        assertFalse(appLock.locked.value)
    }

    @Test
    fun time_away_at_the_limit_locks_again() {
        val appLock = lock().apply { endPrompt(unlocked = true) }
        now = 1_000L
        appLock.onStopped()
        now = 1_000L + AppLock.LOCK_AFTER_MS

        appLock.onStarted()

        assertTrue(appLock.locked.value)
    }

    @Test
    fun a_disabled_lock_never_locks_on_return() {
        val appLock = lock().apply { endPrompt(unlocked = true) }
        now = 1_000L
        appLock.onStopped()
        enabled = false
        now = 1_000L + AppLock.LOCK_AFTER_MS * 2

        appLock.onStarted()

        assertFalse(appLock.locked.value)
    }

    @Test
    fun turning_the_lock_off_unlocks_on_return() {
        val appLock = lock()
        enabled = false

        appLock.onStarted()

        assertFalse(appLock.locked.value)
    }

    @Test
    fun time_spent_in_a_prompt_does_not_count_as_time_away() {
        val appLock = lock().apply { endPrompt(unlocked = true) }
        now = 1_000L
        appLock.onStopped()
        appLock.beginPrompt()
        now = 1_000L + AppLock.LOCK_AFTER_MS * 3
        appLock.onStopped()

        appLock.onStarted()
        appLock.endPrompt(unlocked = false)

        assertFalse(appLock.locked.value)
    }

    @Test
    fun a_passed_prompt_unlocks_and_a_failed_one_keeps_the_lock() {
        val appLock = lock()
        appLock.beginPrompt()
        appLock.endPrompt(unlocked = false)
        assertTrue(appLock.locked.value)

        appLock.beginPrompt()
        appLock.endPrompt(unlocked = true)

        assertFalse(appLock.locked.value)
    }
}
