package app.tether.core

import app.tether.core.generated.CoreEventView
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/**
 * Fans core events out to Kotlin collectors. It replays recent events so a screen that starts
 * after the core still sees `Ready`.
 */
class EventBus(
    replay: Int = DEFAULT_REPLAY,
) {
    private val flow =
        MutableSharedFlow<CoreEventView>(
            replay = replay,
            extraBufferCapacity = EXTRA_BUFFER,
            onBufferOverflow = BufferOverflow.DROP_OLDEST,
        )

    /** Events in arrival order. */
    val events: SharedFlow<CoreEventView> = flow.asSharedFlow()

    /** Publishes one event. Never suspends; the oldest event drops if collectors fall behind. */
    fun publish(event: CoreEventView) {
        flow.tryEmit(event)
    }

    private companion object {
        const val DEFAULT_REPLAY = 16
        const val EXTRA_BUFFER = 64
    }
}
