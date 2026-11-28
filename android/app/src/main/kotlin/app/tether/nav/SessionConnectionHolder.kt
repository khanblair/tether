package app.tether.nav

import androidx.lifecycle.ViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** What opening a session gave back. */
internal sealed interface SessionOpen<out C> {
    /** The connection is still opening. */
    data object Opening : SessionOpen<Nothing>

    /** The session opened. */
    class Opened<C>(
        val connection: C,
    ) : SessionOpen<C>

    /**
     * The session cannot be opened. [reason] is the exception's simple class name, or null when the
     * core reports the session as unavailable.
     */
    class Unavailable(
        val reason: String?,
    ) : SessionOpen<Nothing>
}

/**
 * Holds a session's connection for as long as its back stack entry lives. The transcript screen
 * leaves composition when another screen opens on top of it, such as a server preview. The
 * connection must outlive that, because it owns the session's background processes. It closes
 * when the entry leaves the back stack, and not before.
 *
 * [open] runs through [launch], which must not be tied to this model's own lifetime, so a
 * connection that finishes opening after the entry is gone still gets closed through [release].
 */
internal class SessionConnectionHolder<C : Any>(
    sessionId: String?,
    private val release: (C) -> Unit,
    launch: (suspend () -> Unit) -> Unit,
    open: suspend (String) -> SessionOpen<C>,
) : ViewModel() {
    private val mutableState =
        MutableStateFlow<SessionOpen<C>>(
            if (sessionId == null) SessionOpen.Unavailable(null) else SessionOpen.Opening,
        )
    private var cleared = false

    /** The open state: opening, the connection, or why it is unavailable. */
    val state: StateFlow<SessionOpen<C>> = mutableState.asStateFlow()

    init {
        if (sessionId != null) {
            launch { publish(open(sessionId)) }
        }
    }

    @Synchronized
    private fun publish(result: SessionOpen<C>) {
        if (!cleared) {
            mutableState.value = result
        } else if (result is SessionOpen.Opened) {
            release(result.connection)
        }
    }

    @Synchronized
    override fun onCleared() {
        cleared = true
        (mutableState.value as? SessionOpen.Opened)?.let { release(it.connection) }
    }
}
