package app.tether.nav

import app.tether.session.StartResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class StartMessagesTest {
    private val messages =
        StartMessages(
            noKey = "no key",
            addKey = "Add key",
            noModels = "no models",
            openProviders = "Open Providers",
            failed = "failed",
        )

    @Test
    fun a_started_session_has_no_notice() {
        assertNull(messages.noticeFor(StartResult.Started("session")))
    }

    @Test
    fun a_missing_key_or_model_offers_providers() {
        assertEquals(StartNotice("no key", "Add key"), messages.noticeFor(StartResult.NoKey))
        assertEquals(
            StartNotice("no models", "Open Providers"),
            messages.noticeFor(StartResult.NoModels),
        )
    }

    @Test
    fun a_failed_start_says_so_with_no_action() {
        assertEquals(StartNotice("failed", null), messages.noticeFor(StartResult.Failed))
    }
}
