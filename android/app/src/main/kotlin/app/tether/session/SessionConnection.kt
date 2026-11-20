package app.tether.session

import app.tether.core.generated.CoreEventView
import app.tether.core.generated.EventSink
import app.tether.core.generated.MessageView
import app.tether.core.generated.SecretVault
import app.tether.core.generated.SessionRunner
import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.SessionRuntimeException
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow

/** Keeps the actor alive for the screen, so queued sends and interrupts reach the same turn. */
class SessionConnection(
    private val databasePath: String,
    vault: SecretVault,
    private val sessionId: String,
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
    private val runner: SessionRunner =
        try {
            runtime.openSession(sessionId)
        } catch (error: SessionRuntimeException) {
            runtime.close()
            throw error
        }
    val modelId: String =
        try {
            runtime.loadSession(sessionId).modelId
        } catch (error: SessionRuntimeException) {
            runner.close()
            runtime.close()
            throw error
        }

    fun transcript(): List<MessageView> = runtime.transcript(sessionId)

    fun status(): SessionStatus = readSessionStatus(databasePath, sessionId)

    fun send(text: String) {
        runner.sendMessage(text)
    }

    fun stop() {
        runner.interrupt()
    }

    override fun close() {
        runner.close()
        runtime.close()
    }
}
