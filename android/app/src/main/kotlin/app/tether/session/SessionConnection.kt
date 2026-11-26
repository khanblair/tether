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
import app.tether.session.compose.CommandsGateway
import app.tether.session.compose.CoreCommandsGateway
import app.tether.session.compose.SkippedMention
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

    /** The project this session belongs to. */
    val projectId: String =
        try {
            runtime.loadSession(sessionId).projectId
        } catch (error: SessionRuntimeException) {
            runner.close()
            runtime.close()
            throw error
        }

    /**
     * Model, mode, rename, and usage calls. They use this runtime, not a second one, because the
     * core drops a session's runner from the runtime that changed it.
     */
    val settings: SessionSettingsGateway = CoreSessionSettingsGateway(lazyOf(runtime))

    /** The model of this session, found once per provider and id for the snapshot. */
    internal val modelLookup = ModelLookup()

    /** The composer's slash commands: the command sheet and command resolution. */
    val commands: CommandsGateway = CoreCommandsGateway(lazyOf(runtime))

    /** The agent-loop calls: approvals, questions, checklist, plan, budget, and interruptions. */
    val agent: AgentLoopGateway = CoreAgentLoopGateway(lazyOf(runtime))

    /** Rewinds run on this connection's runner, so their events reach [changes]. */
    val rewinder: SessionRewinder = CoreSessionRewinder { runner }

    fun transcript(): List<MessageView> = runtime.transcript(sessionId)

    fun status(): SessionStatus = readSessionStatus(databasePath, sessionId)

    /** Sends a turn and returns the `@` mentions the core could not attach, with the reasons. */
    fun send(text: String): List<SkippedMention> {
        val outcome = runner.sendMessageWithOutcome(text)
        return outcome.skippedPaths.zip(outcome.skippedReasons, ::SkippedMention)
    }

    /** Hides earlier messages from future context. The transcript keeps them. */
    fun clearContext() {
        runner.clearContext()
    }

    /** The whole session as Markdown, for saving to a file. */
    fun exportMarkdown(): String = runtime.exportSession(sessionId)

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

    /**
     * Opens the runner. With a workspace root, it also turns interactive approvals on. When the
     * workspace is unavailable, such as a missing project folder, it opens plain chat instead.
     */
    private fun openRunner(): SessionRunner {
        val root = workspacesRoot ?: return runtime.openSession(sessionId)
        return openWithFallback(
            workspace = {
                runtime.openSessionWithWorkspace(sessionId, root).also {
                    runtime.setInteractiveApprovals(true)
                }
            },
            plain = { runtime.openSession(sessionId) },
        )
    }
}

/**
 * Runs [workspace]. Only [SessionRuntimeException.Unavailable] falls back to [plain]; every other
 * error reaches the caller unchanged.
 */
internal fun <T> openWithFallback(
    workspace: () -> T,
    plain: () -> T,
): T =
    try {
        workspace()
    } catch (_: SessionRuntimeException.Unavailable) {
        plain()
    }
