package app.tether.session.servers

import app.tether.core.generated.CoreEventView
import app.tether.core.generated.ProcessStatusView
import app.tether.core.generated.ProcessView
import app.tether.preview.PreviewTarget
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** The state of one session-owned process, reduced to what the Android UI needs. */
data class ServerProcess(
    val id: String,
    val command: String,
    val status: ServerStatus,
    val port: UShort?,
)

/** A process status that is independent of the generated core bindings. */
sealed interface ServerStatus {
    data object Running : ServerStatus

    data class Exited(
        val code: Int,
    ) : ServerStatus

    data object Stopped : ServerStatus
}

/** A process state change used by the pure state reducer and its tests. */
data class ServerProcessChange(
    val processId: String,
    val status: ServerStatus,
    val port: UShort?,
)

/** A local preview requested by the agent. */
sealed interface ServerPreviewTarget {
    data class File(
        val path: String,
    ) : ServerPreviewTarget

    data class Server(
        val url: String,
        val processId: String,
    ) : ServerPreviewTarget
}

/** The session's process table and exit notices waiting to be shown once. */
data class ServerState(
    val processes: List<ServerProcess> = emptyList(),
    val pendingExitCodes: List<Int> = emptyList(),
) {
    val running: List<ServerProcess>
        get() = processes.filter { it.status == ServerStatus.Running }

    /** Chooses a port label for one known server, or a count for several or unknown ports. */
    fun barLabel(): ServerBarLabel {
        val active = running
        val port = active.singleOrNull()?.port
        return if (port != null) ServerBarLabel.Port(port) else ServerBarLabel.Count(active.size)
    }

    fun apply(change: ServerProcessChange): ServerState {
        val old = processes.firstOrNull { it.id == change.processId }
        val next =
            processes
                .map { process ->
                    if (process.id == change.processId) {
                        process.copy(status = change.status, port = change.port)
                    } else {
                        process
                    }
                }.let { updated ->
                    if (old == null) {
                        updated + ServerProcess(change.processId, "", change.status, change.port)
                    } else {
                        updated
                    }
                }
        val exitCode = (change.status as? ServerStatus.Exited)?.code
        val newNotice =
            exitCode?.takeIf { it != 0 && old?.status != change.status }
        return copy(
            processes = next,
            pendingExitCodes = pendingExitCodes + listOfNotNull(newNotice),
        )
    }

    fun takeExitNotice(): Pair<Int?, ServerState> =
        pendingExitCodes.firstOrNull() to copy(pendingExitCodes = pendingExitCodes.drop(1))

    companion object {
        fun from(processes: List<ProcessView>): ServerState =
            ServerState(processes.map(::serverProcess))
    }
}

/** The pure label choice used by the process bar. Resources supply the final wording. */
sealed interface ServerBarLabel {
    data class Port(
        val port: UShort,
    ) : ServerBarLabel

    data class Count(
        val count: Int,
    ) : ServerBarLabel
}

/** A small holder for the screen. The reducer above remains usable without Compose or Android. */
class ServerStateHolder {
    private val mutableState = MutableStateFlow(ServerState())
    private var loaded = false

    val state: StateFlow<ServerState> = mutableState.asStateFlow()

    fun loadOnce(processes: List<ProcessView>) {
        if (!loaded) {
            loaded = true
            mutableState.value = ServerState.from(processes)
        }
    }

    fun onProcessChanged(change: ServerProcessChange): Int? {
        mutableState.value = mutableState.value.apply(change)
        val (notice, consumed) = mutableState.value.takeExitNotice()
        mutableState.value = consumed
        return notice
    }
}

/** A process status registry keeps a server preview updated while its session is off screen. */
object ServerProcessRegistry {
    private val mutableStatuses = MutableStateFlow<Map<String, ServerStatus>>(emptyMap())
    private val owners = mutableMapOf<String, String>()

    val statuses: StateFlow<Map<String, ServerStatus>> = mutableStatuses.asStateFlow()

    @Synchronized
    fun register(
        sessionId: String,
        processes: List<ProcessView>,
    ) {
        val next = mutableStatuses.value.toMutableMap()
        processes.forEach { process ->
            owners[process.id] = sessionId
            next[process.id] = process.status.toServerStatus()
        }
        mutableStatuses.value = next
    }

    @Synchronized
    fun onEvent(event: CoreEventView) {
        val changed = event as? CoreEventView.ProcessChanged ?: return
        owners[changed.processId] = changed.sessionId
        mutableStatuses.value =
            mutableStatuses.value +
            (changed.processId to changed.status.toServerStatus())
    }

    @Synchronized
    fun close(sessionId: String) {
        val next = mutableStatuses.value.toMutableMap()
        owners.filterValues { it == sessionId }.keys.forEach { processId ->
            next[processId] = ServerStatus.Stopped
            owners.remove(processId)
        }
        mutableStatuses.value = next
    }

    @Synchronized
    fun clearForTests() {
        owners.clear()
        mutableStatuses.value = emptyMap()
    }
}

fun CoreEventView.processChange(): ServerProcessChange? {
    val changed = this as? CoreEventView.ProcessChanged ?: return null
    return ServerProcessChange(
        processId = changed.processId,
        status = changed.status.toServerStatus(),
        port = changed.port,
    )
}

fun CoreEventView.serverPreviewTarget(): ServerPreviewTarget? {
    val requested = this as? CoreEventView.PreviewRequested ?: return null
    return when (val target = requested.target) {
        is app.tether.core.generated.PreviewTargetView.File ->
            ServerPreviewTarget.File(target.path)
        is app.tether.core.generated.PreviewTargetView.Server ->
            ServerPreviewTarget.Server(target.url, target.processId)
    }
}

private fun serverProcess(process: ProcessView): ServerProcess =
    ServerProcess(
        id = process.id,
        command = process.command,
        status = process.status.toServerStatus(),
        port = process.port,
    )

private fun ProcessStatusView.toServerStatus(): ServerStatus =
    when (this) {
        ProcessStatusView.Running -> ServerStatus.Running
        is ProcessStatusView.Exited -> ServerStatus.Exited(code)
        ProcessStatusView.Stopped -> ServerStatus.Stopped
    }

fun ServerPreviewTarget.toPreviewTarget(projectId: String): PreviewTarget =
    when (this) {
        is ServerPreviewTarget.File -> PreviewTarget.File(projectId, path)
        is ServerPreviewTarget.Server -> PreviewTarget.Server(url, processId)
    }
