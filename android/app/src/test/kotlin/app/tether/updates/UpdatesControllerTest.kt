package app.tether.updates

import app.tether.core.generated.UpdateCheckView
import app.tether.core.generated.UpdaterException
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

private const val HOUR_MS = 60L * 60L * 1000L

@OptIn(ExperimentalCoroutinesApi::class)
class UpdatesControllerTest {
    private val gateway = FakeUpdateGateway()
    private val store = FakeUpdateStore()
    private var now = 100L * HOUR_MS

    private fun TestScope.controller(
        installer: FakeUpdateInstaller = FakeUpdateInstaller(),
        fileExists: (String) -> Boolean = { true },
    ): UpdatesController =
        UpdatesController(
            gateway = gateway,
            installer = installer,
            store = store,
            currentVersion = "0.0.2",
            scope = this,
            io = StandardTestDispatcher(testScheduler),
            clock = { now },
            fileExists = fileExists,
        )

    private fun newer() = UpdateCheckView.Available(releaseView())

    @Test
    fun a_manual_check_shows_a_newer_release_and_remembers_it() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            assertEquals(UpdatePhase.Checking, c.state.value.phase)
            advanceUntilIdle()
            val state = c.state.value
            assertEquals(UpdatePhase.Available, state.phase)
            assertEquals("0.0.3", state.release?.version)
            assertEquals("Notes for 0.0.3", state.release?.notes)
            assertEquals(1_000L, state.release?.apkSizeBytes)
            assertEquals("0.0.3", state.newerVersion)
            assertTrue(state.updateAvailable)
            assertEquals(now, store.last)
            assertEquals("0.0.3", store.available)
            assertEquals(listOf("check:0.0.2"), gateway.calls)
        }

    @Test
    fun an_up_to_date_result_clears_what_was_remembered() =
        runTest {
            store.available = "0.0.3"
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            assertEquals(UpdatePhase.UpToDate, c.state.value.phase)
            assertFalse(c.state.value.updateAvailable)
            assertNull(store.available)
            assertEquals(now, store.last)
        }

    @Test
    fun a_failed_manual_check_shows_the_problem_and_a_silent_one_does_not() =
        runTest {
            val c = controller()
            gateway.failNext = UpdaterException.Network()
            c.checkNow()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Failed, c.state.value.phase)
            assertEquals(UpdateProblem.Network, c.state.value.problem)

            val quiet = controller()
            gateway.failNext = UpdaterException.Network()
            quiet.checkIfDue()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Idle, quiet.state.value.phase)
            assertNull(quiet.state.value.problem)
        }

    @Test
    fun the_background_check_runs_only_when_twelve_hours_have_passed() =
        runTest {
            val c = controller()
            store.last = now - 11 * HOUR_MS
            c.checkIfDue()
            advanceUntilIdle()
            assertTrue(gateway.calls.isEmpty())
            store.last = now - 12 * HOUR_MS
            c.checkIfDue()
            advanceUntilIdle()
            assertEquals(listOf("check:0.0.2"), gateway.calls)
        }

    @Test
    fun the_background_check_runs_when_none_ever_ran() =
        runTest {
            val c = controller()
            c.checkIfDue()
            advanceUntilIdle()
            assertEquals(1, gateway.calls.size)
        }

    @Test
    fun restore_shows_a_newer_remembered_version_and_ignores_an_old_one() =
        runTest {
            store.available = "0.0.3"
            val c = controller()
            c.restore()
            advanceUntilIdle()
            assertEquals("0.0.3", c.state.value.knownNewerVersion)
            assertTrue(c.state.value.updateAvailable)

            store.available = "0.0.2"
            val old = controller()
            old.restore()
            advanceUntilIdle()
            assertNull(old.state.value.knownNewerVersion)
            assertFalse(old.state.value.updateAvailable)
        }

    @Test
    fun a_check_is_refused_while_a_download_runs() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            c.download()
            gateway.calls.clear()
            c.checkNow()
            advanceUntilIdle()
            assertFalse(gateway.calls.any { it.startsWith("check") })
        }

    @Test
    fun download_reports_progress_and_ends_ready_with_the_verified_path() =
        runTest {
            gateway.checkResult = newer()
            gateway.progressSteps = listOf(0UL to 1_000UL, 500UL to 1_000UL, 1_000UL to 1_000UL)
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            c.download()
            assertEquals(UpdatePhase.Downloading, c.state.value.phase)
            assertEquals(1_000L, c.state.value.totalBytes)
            advanceUntilIdle()
            val state = c.state.value
            assertEquals(UpdatePhase.Ready, state.phase)
            assertEquals("/files/tether-full-v0.0.3.apk", state.apkPath)
            assertEquals(100, state.percent)
            assertTrue("download:0.0.3" in gateway.calls)
        }

    @Test
    fun a_download_needs_an_available_release() =
        runTest {
            val c = controller()
            c.download()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Idle, c.state.value.phase)
            assertTrue(gateway.calls.isEmpty())
        }

    @Test
    fun a_bad_checksum_fails_the_download_and_allows_another_try() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            gateway.failNext = UpdaterException.Checksum()
            c.download()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Failed, c.state.value.phase)
            assertEquals(UpdateProblem.ChecksumMismatch, c.state.value.problem)
            assertNull(c.state.value.apkPath)
            c.download()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Ready, c.state.value.phase)
        }

    @Test
    fun a_cancel_tapped_before_the_download_starts_never_reaches_the_core() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            c.download()
            c.cancelDownload()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Available, c.state.value.phase)
            assertNull(c.state.value.problem)
            assertNull(c.state.value.apkPath)
            assertFalse(gateway.calls.any { it.startsWith("download") })
        }

    @Test
    fun a_cancel_that_comes_too_late_to_stop_the_call_still_discards_the_result() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            gateway.onDownload = { c.cancelDownload() }
            c.download()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Available, c.state.value.phase)
            assertNull(c.state.value.apkPath)
            assertTrue(gateway.calls.any { it.startsWith("download") })
        }

    @Test
    fun a_download_that_the_core_reports_as_cancelled_goes_back_to_available() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            gateway.failNext = UpdaterException.Cancelled()
            c.download()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Available, c.state.value.phase)
            assertNull(c.state.value.problem)
        }

    @Test
    fun a_new_download_after_a_cancel_is_not_cancelled() =
        runTest {
            gateway.checkResult = newer()
            val c = controller()
            c.checkNow()
            advanceUntilIdle()
            c.download()
            c.cancelDownload()
            advanceUntilIdle()
            c.download()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Ready, c.state.value.phase)
        }

    @Test
    fun cancel_does_nothing_when_no_download_runs() =
        runTest {
            val c = controller()
            c.cancelDownload()
            advanceUntilIdle()
            assertFalse("cancel" in gateway.calls)
        }

    private fun TestScope.readyController(
        installer: FakeUpdateInstaller,
        fileExists: (String) -> Boolean = { true },
    ): UpdatesController {
        gateway.checkResult = newer()
        val c = controller(installer, fileExists)
        c.checkNow()
        advanceUntilIdle()
        c.download()
        advanceUntilIdle()
        return c
    }

    @Test
    fun install_hands_the_verified_file_to_the_system_and_ends_installed() =
        runTest {
            val installer = FakeUpdateInstaller()
            val c = readyController(installer)
            c.install()
            assertEquals(UpdatePhase.Installing, c.state.value.phase)
            advanceUntilIdle()
            assertEquals(UpdatePhase.Installed, c.state.value.phase)
            assertEquals(listOf("install:/files/tether-full-v0.0.3.apk"), installer.calls)
        }

    @Test
    fun install_asks_for_permission_first_and_continues_after_it_is_granted() =
        runTest {
            val installer = FakeUpdateInstaller().apply { canInstall = false }
            val c = readyController(installer)
            c.install()
            assertEquals(UpdatePhase.NeedsPermission, c.state.value.phase)
            c.openInstallSettings()
            assertEquals(listOf("settings"), installer.calls)
            c.onInstallPermissionResult()
            assertEquals(UpdatePhase.NeedsPermission, c.state.value.phase)
            installer.canInstall = true
            c.onInstallPermissionResult()
            assertEquals(UpdatePhase.Ready, c.state.value.phase)
            c.install()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Installed, c.state.value.phase)
        }

    @Test
    fun every_install_failure_maps_to_its_own_problem_and_keeps_the_file_for_a_retry() =
        runTest {
            val expected =
                mapOf(
                    InstallFailure.Cancelled to UpdateProblem.InstallDeclined,
                    InstallFailure.Blocked to UpdateProblem.InstallBlocked,
                    InstallFailure.Conflict to UpdateProblem.InstallConflict,
                    InstallFailure.Incompatible to UpdateProblem.InstallIncompatible,
                    InstallFailure.Invalid to UpdateProblem.InstallInvalid,
                    InstallFailure.Storage to UpdateProblem.InstallStorage,
                    InstallFailure.Other to UpdateProblem.InstallFailed,
                )
            assertEquals(InstallFailure.entries.toSet(), expected.keys)
            expected.forEach { (failure, problem) ->
                val installer = FakeUpdateInstaller().apply { outcome = { it.onFailure(failure) } }
                val c = readyController(installer)
                c.install()
                advanceUntilIdle()
                assertEquals(failure.name, UpdatePhase.Failed, c.state.value.phase)
                assertEquals(failure.name, problem, c.state.value.problem)
                assertEquals("/files/tether-full-v0.0.3.apk", c.state.value.apkPath)
                installer.outcome = { it.onSuccess() }
                c.install()
                advanceUntilIdle()
                assertEquals(failure.name, UpdatePhase.Installed, c.state.value.phase)
            }
        }

    @Test
    fun an_install_that_throws_is_a_failure_not_a_crash() =
        runTest {
            val installer =
                FakeUpdateInstaller().apply {
                    throwOnInstall =
                        IllegalStateException("boom")
                }
            val c = readyController(installer)
            c.install()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Failed, c.state.value.phase)
            assertEquals(UpdateProblem.InstallFailed, c.state.value.problem)
        }

    @Test
    fun the_play_build_opens_the_store_and_never_installs() =
        runTest {
            val installer = FakeUpdateInstaller(installsInApp = false)
            val c = readyController(installer)
            c.install()
            advanceUntilIdle()
            assertEquals(listOf("store"), installer.calls)
            assertEquals(UpdatePhase.Ready, c.state.value.phase)
        }

    @Test
    fun a_missing_download_is_forgotten_and_offered_again_instead_of_retried() =
        runTest {
            val installer = FakeUpdateInstaller()
            val c = readyController(installer, fileExists = { false })
            c.install()
            advanceUntilIdle()
            assertEquals(UpdatePhase.Failed, c.state.value.phase)
            assertEquals(UpdateProblem.DownloadMissing, c.state.value.problem)
            assertNull(c.state.value.apkPath)
            assertTrue(installer.calls.isEmpty())
            c.download()
            assertEquals(UpdatePhase.Downloading, c.state.value.phase)
        }

    @Test
    fun a_file_that_vanishes_while_the_install_fails_is_reported_as_missing() =
        runTest {
            var present = true
            val installer =
                FakeUpdateInstaller().apply {
                    outcome = {
                        present = false
                        it.onFailure(InstallFailure.Storage)
                    }
                }
            val c = readyController(installer, fileExists = { present })
            c.install()
            advanceUntilIdle()
            assertEquals(UpdateProblem.DownloadMissing, c.state.value.problem)
            assertNull(c.state.value.apkPath)
        }

    @Test
    fun a_failed_background_check_counts_for_the_throttle_and_keeps_the_known_version() =
        runTest {
            store.available = "0.0.3"
            val c = controller()
            c.restore()
            advanceUntilIdle()
            gateway.failNext = UpdaterException.Network()
            c.checkIfDue()
            advanceUntilIdle()
            assertEquals(now, store.last)
            assertEquals("0.0.3", store.available)
            assertEquals(1, gateway.calls.count { it.startsWith("check") })
            c.checkIfDue()
            advanceUntilIdle()
            assertEquals(1, gateway.calls.count { it.startsWith("check") })
        }

    @Test
    fun a_failed_manual_check_is_recorded_too() =
        runTest {
            val c = controller()
            gateway.failNext = UpdaterException.Network()
            c.checkNow()
            advanceUntilIdle()
            assertEquals(now, store.last)
            assertNull(store.available)
        }

    @Test
    fun problems_map_from_every_core_error() {
        assertEquals(UpdateProblem.Network, problemFor(UpdaterException.Network()))
        assertEquals(UpdateProblem.NothingPublished, problemFor(UpdaterException.Unavailable()))
        assertEquals(UpdateProblem.ChecksumMismatch, problemFor(UpdaterException.Checksum()))
        assertEquals(UpdateProblem.TooLarge, problemFor(UpdaterException.TooLarge()))
        assertEquals(UpdateProblem.Storage, problemFor(UpdaterException.Storage()))
        assertEquals(UpdateProblem.BadResponse, problemFor(UpdaterException.Invalid()))
        assertEquals(UpdateProblem.BadResponse, problemFor(UpdaterException.NoRelease()))
        assertEquals(UpdateProblem.BadResponse, problemFor(IllegalStateException("x")))
    }

    @Test
    fun percent_is_unknown_without_a_size_and_never_passes_a_hundred() {
        val state = UpdatesUiState(currentVersion = "0.0.2", installsInApp = true)
        assertNull(state.percent)
        assertEquals(50, state.copy(downloadedBytes = 500L, totalBytes = 1_000L).percent)
        assertEquals(100, state.copy(downloadedBytes = 2_000L, totalBytes = 1_000L).percent)
    }
}
