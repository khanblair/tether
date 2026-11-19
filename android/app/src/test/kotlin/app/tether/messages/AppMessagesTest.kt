package app.tether.messages

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class AppMessagesTest {
    @Test
    fun a_collector_receives_what_show_emits_with_its_action_label() =
        runTest {
            val messages = AppMessages()
            val received = mutableListOf<AppMessage>()
            val job =
                launch(UnconfinedTestDispatcher(testScheduler)) {
                    messages.flow.collect { received += it }
                }
            messages.show("Test event sent", "Undo")
            messages.show("Saved")
            job.cancel()
            assertEquals(
                listOf(AppMessage("Test event sent", "Undo"), AppMessage("Saved")),
                received,
            )
        }
}
