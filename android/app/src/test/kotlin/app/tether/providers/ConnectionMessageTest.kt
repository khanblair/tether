package app.tether.providers

import app.tether.R
import app.tether.core.generated.ConnectionStatusView
import app.tether.design.components.BannerTone
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ConnectionMessageTest {
    @Test
    fun an_accepted_or_untested_status_has_no_message() {
        assertNull(connectionMessage(ConnectionStatusView.OK))
        assertNull(connectionMessage(ConnectionStatusView.UNSET))
        assertNull(connectionMessage(ConnectionStatusView.TESTING))
    }

    @Test
    fun a_rejected_key_is_a_danger_message() {
        val message = connectionMessage(ConnectionStatusView.BAD_KEY)

        assertEquals(R.string.provider_detail_rejected_title, message?.title)
        assertEquals(R.string.provider_detail_rejected_body, message?.body)
        assertEquals(BannerTone.Danger, message?.tone)
    }

    @Test
    fun a_network_error_is_a_warning_message() {
        val message = connectionMessage(ConnectionStatusView.NETWORK_ERROR)

        assertEquals(R.string.provider_detail_offline_title, message?.title)
        assertEquals(BannerTone.Warning, message?.tone)
    }

    @Test
    fun a_block_by_region_is_a_danger_message() {
        val message = connectionMessage(ConnectionStatusView.BLOCKED)

        assertEquals(R.string.provider_detail_blocked_title, message?.title)
        assertEquals(BannerTone.Danger, message?.tone)
    }
}
