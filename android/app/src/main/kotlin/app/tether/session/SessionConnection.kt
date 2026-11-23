package app.tether.session

import app.tether.core.generated.CoreEventView
import app.tether.core.generated.EventSink
import app.tether.core.generated.MessageView
import app.tether.core.generated.SecretVault
import app.tether.core.generated.SessionRunner
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.SessionRuntimeException
import app.tether.session.agent.AgentLoopGateway
import app.tether.session.agent.CoreAgentLoopGateway
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow

/**
 * Keeps the actor alive for the screen, so queued sends and interrupts reach the same turn. With
 * no [workspacesRoot] the session opens for plain chat: no workspace and no tools. With one, it
 * opens with its workspace and tools, and approvals reach the app.
 */
class SessionConnection(
    private val databasePath: String,
    vault: SecretVault,
    val sessionId: String,
    private val workspacesRoot: String? = null,
) : AutoCloseable {
    private val events = MutableSharedFlow<CoreEventView>(extraBufferCapacity = 64)
    val changes: SharedFlow<CoreEventView> = events
    private val sink =
        object : EventSink {
            override fun onEvent(event: CoreEventView) {
                events.tryEmit(event)
            }
        }
    private val runtime = SessionRuntime(databasePath, vault, sink)

    @Volatile
    private var runner: SessionRunner =
        try {
            openRunner()
        } catch (error: SessionRuntimeException) {
            runtime.close()
            throw error
        }

    /**
     * Model, mode, rename, and usage calls. They use this runtime, not a second one, because the
     * core drops a session's runner from the runtime that changed it.
     */
    val settings: SessionSettingsGateway = CoreSessionSettingsGateway(lazyOf(runtime))

    /** The agent-loop calls: approvals, questions, checklist, plan, budget, and interruptions. */
    val agent: AgentLoopGateway = CoreAgentLoopGateway(lazyOf(runtime))

    /** Rewinds run on this connection's runner, so their events reach [changes]. */
    val rewinder: SessionRewinder = CoreSessionRewinder { runner }

    fun transcript(): List<MessageView> = runtime.transcript(sessionId)

    fun status(): SessionStatus = readSessionStatus(databasePath, sessionId)

    fun send(text: String) {
        runner.sendMessage(text)
    }

    fun stop() {
        runner.interrupt()
    }

    /**
     * Replaces the runner after a model or mode change. The core drops the old runner then, so
     * the next turn uses the new model or mode. Call it off the main thread, with no turn running.
     */
    fun reopen() {
        val old = runner
        runner = coreCall { openRunner() }
        old.close()
    }

    override fun close() {
        runner.close()
        runtime.close()
    }

    /** Opens the runner. With a workspace root, it also turns interactive approvals on. */
    private fun openRunner(): SessionRunner {
        val root = workspacesRoot ?: return runtime.openSession(sessionId)
        val opened = runtime.openSessionWithWorkspace(sessionId, root)
        runtime.setInteractiveApprovals(true)
        return opened
    }
}
