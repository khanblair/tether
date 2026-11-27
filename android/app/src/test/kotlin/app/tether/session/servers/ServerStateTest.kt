package app.tether.session.servers

import app.tether.core.generated.ProcessStatusView
import app.tether.core.generated.ProcessView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ServerStateTest {
    @Test
    fun list_load_and_events_cover_start_port_exit_stop_and_a_second_process() {
        val first = process("one", ProcessStatusView.Running, null)
        val state = ServerState.from(listOf(first))
        assertEquals(listOf("one"), state.running.map { it.id })

        val withPort =
            state.apply(ServerProcessChange("one", ServerStatus.Running, 5173u.toUShort()))
        assertEquals(5173u.toUShort(), withPort.running.single().port)
        assertEquals(ServerBarLabel.Port(5173u.toUShort()), withPort.barLabel())

        val second =
            withPort.apply(ServerProcessChange("two", ServerStatus.Running, 4173u.toUShort()))
        assertEquals(setOf("one", "two"), second.running.map { it.id }.toSet())
        assertEquals(ServerBarLabel.Count(2), second.barLabel())

        val exited = second.apply(ServerProcessChange("one", ServerStatus.Exited(2), null))
        assertEquals(2, exited.pendingExitCodes.single())
        assertEquals(listOf("two"), exited.running.map { it.id })

        val stopped = exited.apply(ServerProcessChange("two", ServerStatus.Stopped, null))
        assertTrue(stopped.running.isEmpty())
    }

    @Test
    fun the_holder_shows_a_nonzero_exit_once() {
        val holder = ServerStateHolder()
        holder.loadOnce(emptyList())
        assertEquals(
            3,
            holder.onProcessChanged(ServerProcessChange("one", ServerStatus.Exited(3), null)),
        )
        assertEquals(
            null,
            holder.onProcessChanged(ServerProcessChange("one", ServerStatus.Exited(3), null)),
        )
    }

    private fun process(
        id: String,
        status: ProcessStatusView,
        port: UShort?,
    ) = ProcessView(id, "npm run dev", status, port, 0uL)
}
