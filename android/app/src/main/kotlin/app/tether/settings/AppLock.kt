package app.tether.settings

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * Decides when the app shows its lock screen. The app locks at start when the lock is on, and again
 * after it has been in the background for [LOCK_AFTER_MS]. Time spent in a biometric prompt does
 * not
 * count, because the prompt can stop the activity.
 */
class AppLock(
    private val isEnabled: () -> Boolean,
    private val clock: () -> Long,
) {
    private val mutableLocked = MutableStateFlow(isEnabled())
    private var leftAt: Long? = null
    private var prompting = false

    /** True while the lock screen must cover the app. */
    val locked: StateFlow<Boolean> = mutableLocked.asStateFlow()

    /** The activity left the foreground. Starts the timer unless a prompt is open. */
    fun onStopped() {
        if (!prompting) leftAt = clock()
    }

    /** The activity came back. Locks when the lock is on and the app was away long enough. */
    fun onStarted() {
        val left = leftAt
        leftAt = null
        if (!isEnabled()) {
            mutableLocked.value = false
        } else if (!prompting && left != null && clock() - left >= LOCK_AFTER_MS) {
            mutableLocked.value = true
        }
    }

    /** A biometric prompt is about to open. The time it takes does not count as time away. */
    fun beginPrompt() {
        prompting = true
    }

    /** The prompt ended, whether it passed or not. A pass unlocks; a failure keeps the lock. */
    fun endPrompt(unlocked: Boolean) {
        prompting = false
        if (unlocked) mutableLocked.value = false
    }

    companion object {
        /** How long the app may stay in the background before it locks again. */
        const val LOCK_AFTER_MS = 30_000L
    }
}
