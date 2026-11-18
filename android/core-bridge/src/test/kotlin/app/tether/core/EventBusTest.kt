package app.tether.core

import app.tether.core.generated.CoreEventView
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

class EventBusTest {
    @Test
    fun late_collector_receives_replayed_event() =
        runTest {
            val bus = EventBus()
            bus.publish(CoreEventView.Ready(version = "0.0.1"))

            val first = bus.events.first()

            assertEquals(CoreEventView.Ready(version = "0.0.1"), first)
        }

    @Test
    fun replay_keeps_only_the_newest_events() =
        runTest {
            val bus = EventBus(replay = 1)
            bus.publish(CoreEventView.Test(message = "old"))
            bus.publish(CoreEventView.Test(message = "new"))

            val first = bus.events.first()

            assertEquals(CoreEventView.Test(message = "new"), first)
        }
}
