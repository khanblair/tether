package app.tether.e2e

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.tether.core.KeystoreSecretsPlatform
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.EventSink
import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProviderSettings
import app.tether.core.generated.ProviderView
import app.tether.core.generated.SecretVault
import app.tether.core.generated.SessionRuntime
import app.tether.core.keystoreSecretVault
import org.junit.After
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import java.io.File
import java.security.KeyStore
import java.util.UUID
import java.util.concurrent.BlockingQueue
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

/**
 * Proves the native session path runs on a device: a project, a stored provider key, a session,
 * and one turn. The key is fake and the turn has no network, so the turn fails with an event, and
 * the test process survives. Run it on a connected device or emulator with `just test-device`.
 */
@RunWith(AndroidJUnit4::class)
class SessionFlowTest {
    private val alias = "tether_secrets_e2e_${UUID.randomUUID()}"
    private lateinit var root: File

    @Before
    fun setUp() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        root = File(context.filesDir, "e2e-${UUID.randomUUID()}")
    }

    @After
    fun tearDown() {
        root.deleteRecursively()
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        store.deleteEntry(alias)
    }

    @Test
    fun a_turn_ends_with_an_event_and_the_session_is_managed() {
        val databasePath = File(root, "tether.db").absolutePath
        val workspaces = File(root, "workspaces").apply { mkdirs() }.absolutePath
        val events = LinkedBlockingQueue<CoreEventView>()

        keystoreSecretVault(File(root, "secrets"), KeystoreSecretsPlatform(alias)).use { vault ->
            storeFakeKey(databasePath, vault)
            ProjectRuntime(databasePath, workspaces).use { projects ->
                val project = projects.createProject("e2e")
                SessionRuntime(databasePath, vault, QueueSink(events)).use { runtime ->
                    runOneTurn(runtime, project.id, workspaces, events)
                }
            }
        }
    }

    private fun storeFakeKey(
        databasePath: String,
        vault: SecretVault,
    ) {
        ProviderSettings(databasePath, vault).use { settings ->
            settings.setKey(ProviderView.ANTHROPIC, FAKE_KEY)
        }
    }

    private fun runOneTurn(
        runtime: SessionRuntime,
        projectId: String,
        workspaces: String,
        events: BlockingQueue<CoreEventView>,
    ) {
        val session = runtime.createSession(projectId, ProviderView.ANTHROPIC, MODEL_ID)
        runtime.openSessionWithWorkspace(session.id, workspaces).use { runner ->
            runtime.setInteractiveApprovals(true)
            runner.sendMessageWithOutcome("hello")
            awaitTurnEnd(events, session.id)
        }
        runtime.renameSession(session.id, "e2e renamed")
        runtime.setSessionMode(session.id, PermissionModeView.ASK)
        runtime.enableBypass(session.id)
    }

    /** Waits for the turn of [sessionId] to finish or fail. Other sessions' events are skipped. */
    private fun awaitTurnEnd(
        events: BlockingQueue<CoreEventView>,
        sessionId: String,
    ) {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(TURN_WAIT_SECONDS)
        while (true) {
            val remaining = deadline - System.nanoTime()
            assertTrue("no turn end within $TURN_WAIT_SECONDS seconds", remaining > 0)
            val event = events.poll(remaining, TimeUnit.NANOSECONDS) ?: continue
            if (endsTurnOf(event, sessionId)) return
        }
    }

    private fun endsTurnOf(
        event: CoreEventView,
        sessionId: String,
    ): Boolean =
        when (event) {
            is CoreEventView.TurnFinished -> event.sessionId == sessionId
            is CoreEventView.TurnFailed -> event.sessionId == sessionId
            else -> false
        }

    /** Hands each core event to [events], in the order the core sent them. */
    private class QueueSink(
        private val events: BlockingQueue<CoreEventView>,
    ) : EventSink {
        override fun onEvent(event: CoreEventView) {
            events.put(event)
        }
    }

    private companion object {
        const val FAKE_KEY = "sk-test-not-real"
        const val MODEL_ID = "claude-haiku-5-5"
        const val TURN_WAIT_SECONDS = 30L
    }
}
