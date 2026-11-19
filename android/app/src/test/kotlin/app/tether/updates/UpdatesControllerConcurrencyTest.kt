package app.tether.updates

import app.tether.core.generated.UpdateCheckView
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger

private const val HOUR_MS = 60L * 60L * 1000L
private const val WAIT_MS = 5_000L

/**
 * Tests of races that a single-threaded test dispatcher cannot show. They use real threads, and
 * every wait has a limit, so a broken controller fails a test instead of hanging it.
 */
class UpdatesControllerConcurrencyTest {
    private val gateway = FakeUpdateGateway()
    private val store = FakeUpdateStore()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    @After
    fun stop() = scope.cancel()

    private fun controller() =
        UpdatesController(
            gateway = gateway,
            installer = FakeUpdateInstaller(),
            store = store,
            currentVersion = "0.0.2",
            scope = scope,
            io = Dispatchers.Default,
            clock = { 100L * HOUR_MS },
            fileExists = { true },
        )

    private fun waitUntil(
        what: String,
        condition: () -> Boolean,
    ) {
        val end = System.currentTimeMillis() + WAIT_MS
        while (!condition()) {
            if (System.currentTimeMillis() > end) fail("timed out waiting for $what")
            Thread.sleep(POLL_MS)
        }
    }

    private fun checkCalls() = gateway.calls.count { it.startsWith("check") }

    @Test
    fun a_cancel_lost_while_the_download_starts_is_asked_again_until_the_call_returns() {
        gateway.checkResult = UpdateCheckView.Available(releaseView())
        val c = controller()
        c.checkNow()
        waitUntil("the release") { c.state.value.phase == UpdatePhase.Available }
        val cancels = AtomicInteger(0)
        val started = CountDownLatch(1)
        val secondCancel = CountDownLatch(1)
        gateway.onDownload = {
            started.countDown()
            secondCancel.await(WAIT_MS, TimeUnit.MILLISECONDS)
        }
        // The first cancel is lost (it throws, like a core with no download registered yet).
        gateway.onCancel = {
            if (cancels.incrementAndGet() ==
                1
            ) {
                error("lost")
            } else {
                secondCancel.countDown()
            }
        }
        c.download()
        assertTrue(started.await(WAIT_MS, TimeUnit.MILLISECONDS))
        c.cancelDownload()
        waitUntil("the download call to return") { c.state.value.phase != UpdatePhase.Downloading }
        assertEquals(UpdatePhase.Available, c.state.value.phase)
        assertNull(c.state.value.apkPath)
        assertTrue("cancel was asked ${cancels.get()} times", cancels.get() >= 2)
    }

    @Test
    fun a_late_background_check_result_does_not_overwrite_a_download() {
        gateway.checkResult = UpdateCheckView.Available(releaseView())
        val c = controller()
        c.checkNow()
        waitUntil("the release") { c.state.value.phase == UpdatePhase.Available }
        store.last = null
        val checkStarted = CountDownLatch(1)
        val releaseCheck = CountDownLatch(1)
        gateway.onCheck = {
            checkStarted.countDown()
            releaseCheck.await(WAIT_MS, TimeUnit.MILLISECONDS)
        }
        c.checkIfDue()
        assertTrue(checkStarted.await(WAIT_MS, TimeUnit.MILLISECONDS))
        val downloadStarted = CountDownLatch(1)
        val releaseDownload = CountDownLatch(1)
        gateway.onDownload = {
            downloadStarted.countDown()
            releaseDownload.await(WAIT_MS, TimeUnit.MILLISECONDS)
        }
        c.download()
        assertTrue(downloadStarted.await(WAIT_MS, TimeUnit.MILLISECONDS))
        releaseCheck.countDown()
        waitUntil("the background check to record its result") { store.last != null }
        Thread.sleep(SETTLE_MS)
        assertEquals(UpdatePhase.Downloading, c.state.value.phase)
        releaseDownload.countDown()
        waitUntil("the download to finish") { c.state.value.phase == UpdatePhase.Ready }
    }

    @Test
    fun a_manual_check_while_a_background_check_runs_does_not_start_a_second_one() {
        val c = controller()
        val started = CountDownLatch(1)
        val release = CountDownLatch(1)
        gateway.onCheck = {
            started.countDown()
            release.await(WAIT_MS, TimeUnit.MILLISECONDS)
        }
        c.checkIfDue()
        assertTrue(started.await(WAIT_MS, TimeUnit.MILLISECONDS))
        c.checkNow()
        Thread.sleep(SETTLE_MS)
        assertEquals(1, checkCalls())
        assertEquals(UpdatePhase.Checking, c.state.value.phase)
        release.countDown()
        waitUntil("the check result") { c.state.value.phase == UpdatePhase.UpToDate }
        assertEquals(1, checkCalls())
    }
}

private const val POLL_MS = 5L
private const val SETTLE_MS = 150L
