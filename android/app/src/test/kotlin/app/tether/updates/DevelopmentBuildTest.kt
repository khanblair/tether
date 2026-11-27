package app.tether.updates

import app.tether.R
import app.tether.core.generated.UpdateCheckView
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DevelopmentBuildTest {
    @Test
    fun a_debuggable_build_has_updates_off() {
        assertFalse(updatesEnabled(isDebuggable = true))
    }

    @Test
    fun a_release_build_has_updates_on() {
        assertTrue(updatesEnabled(isDebuggable = false))
    }

    @Test
    fun about_shows_the_development_build_wording_only_in_a_debuggable_build() {
        val debug =
            UpdatesUiState(
                currentVersion = "0.0.4",
                installsInApp = true,
                developmentBuild = true,
            )
        val release = debug.copy(developmentBuild = false)
        assertEquals(R.string.dev_build_version, versionLabelFor(debug))
        assertEquals(R.string.about_version, versionLabelFor(release))
    }

    @Test
    fun a_development_build_hides_a_stale_update_from_derived_surfaces() {
        val state =
            UpdatesUiState(
                currentVersion = "0.0.4",
                installsInApp = true,
                release =
                    ReleaseSummary(
                        version = "0.0.5",
                        notes = "Notes",
                        publishedAt = "2026-11-20T08:00:00Z",
                        apkSizeBytes = 1_000L,
                    ),
                developmentBuild = true,
            )

        assertFalse(state.updateAvailable)
        assertNull(state.newerVersion)
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class DevelopmentBuildControllerTest {
    private val gateway = FakeUpdateGateway()
    private val store = FakeUpdateStore()
    private val installer = FakeUpdateInstaller()

    private fun TestScope.controller(isDebuggable: Boolean): UpdatesController =
        UpdatesController(
            gateway = gateway,
            installer = installer,
            store = store,
            currentVersion = "0.0.4",
            scope = this,
            io = StandardTestDispatcher(testScheduler),
            clock = { 100L * HOUR_MS },
            fileExists = { true },
            isDebuggable = isDebuggable,
        )

    @Test
    fun a_debuggable_controller_never_calls_its_release_source() =
        runTest {
            gateway.checkResult = UpdateCheckView.Available(releaseView())
            val c = controller(isDebuggable = true)
            c.checkIfDue()
            c.checkNow()
            advanceUntilIdle()
            assertEquals(emptyList<String>(), gateway.calls)
            assertNull(store.last)
            assertEquals(UpdatePhase.Idle, c.state.value.phase)
            assertTrue(c.state.value.developmentBuild)
        }

    @Test
    fun a_debuggable_controller_does_not_offer_a_release_remembered_by_an_earlier_build() =
        runTest {
            store.available = "0.0.5"
            val c = controller(isDebuggable = true)
            c.restore()
            advanceUntilIdle()
            assertNull(c.state.value.knownNewerVersion)
            assertFalse(c.state.value.updateAvailable)
            assertNull(c.state.value.newerVersion)
        }

    @Test
    fun a_release_controller_still_checks() =
        runTest {
            gateway.checkResult = UpdateCheckView.UpToDate("0.0.4", "0.0.4")
            val c = controller(isDebuggable = false)
            c.checkNow()
            advanceUntilIdle()
            assertEquals(listOf("check:0.0.4"), gateway.calls)
            assertEquals(UpdatePhase.UpToDate, c.state.value.phase)
            assertFalse(c.state.value.developmentBuild)
        }

    private companion object {
        const val HOUR_MS = 60L * 60L * 1000L
    }
}
