package app.tether.core

import app.tether.core.generated.CoreEventView
import app.tether.core.generated.EventSink
import app.tether.core.generated.TetherCore
import app.tether.core.generated.version
import kotlinx.coroutines.flow.SharedFlow

/**
 * The app's handle on the Rust core. It turns the core's callbacks into a Flow of events.
 * Create one per process and call [start] once.
 */
class CoreClient(
    private val bus: EventBus = EventBus(),
) {
    private var core: TetherCore? = null

    private val sink =
        object : EventSink {
            override fun onEvent(event: CoreEventView) {
                bus.publish(event)
            }
        }

    /** Events from the core, replayed to late collectors. */
    val events: SharedFlow<CoreEventView> get() = bus.events

    /** Core version. Equals the app version. */
    fun coreVersion(): String = version()

    /** Starts the core. The core emits `Ready` before this returns. */
    @Synchronized
    fun start() {
        if (core == null) core = TetherCore(sink)
    }

    /** Asks the core to emit a `Test` event, to prove the event path. */
    fun emitTestEvent(message: String) {
        checkNotNull(core) { "CoreClient.start() was not called" }.emitTestEvent(message)
    }
}
