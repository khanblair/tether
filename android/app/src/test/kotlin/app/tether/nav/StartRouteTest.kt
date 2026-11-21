package app.tether.nav

import app.tether.settings.FakeSettingsStore
import org.junit.Assert.assertEquals
import org.junit.Test

class StartRouteTest {
    @Test
    fun a_new_install_starts_the_first_run_flow() {
        assertEquals("onboarding", startRoute(FakeSettingsStore(finished = false)))
    }

    @Test
    fun a_finished_or_skipped_flow_starts_at_home() {
        assertEquals("home", startRoute(FakeSettingsStore(finished = true)))
    }
}
