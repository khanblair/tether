package app.tether.linux

import app.tether.core.generated.SessionRuntime
import app.tether.core.generated.LinuxShellSink as CoreLinuxShellSink

/** Thread-safe callback boundary for raw shell output. */
interface LinuxShellSink {
    fun onOutput(bytes: ByteArray)

    fun onExit(code: Int)
}

/** The small handle needed by the screen and ViewModel. */
interface LinuxShellHandle {
    fun write(bytes: ByteArray)

    fun resize(
        rows: Int,
        columns: Int,
    )

    fun close()
}

/** Opens one shell through the core runtime. Calls may block and belong off the main thread. */
interface LinuxShellGateway {
    fun open(
        projectId: String?,
        rows: Int,
        columns: Int,
        sink: LinuxShellSink,
    ): LinuxShellHandle
}

/** Gateway over the lazy UniFFI runtime owned by [app.tether.di.AppModule]. */
class CoreLinuxShellGateway(
    private val runtime: Lazy<SessionRuntime>,
) : LinuxShellGateway {
    override fun open(
        projectId: String?,
        rows: Int,
        columns: Int,
        sink: LinuxShellSink,
    ): LinuxShellHandle {
        val coreSink =
            object : CoreLinuxShellSink {
                override fun onOutput(bytes: ByteArray) {
                    sink.onOutput(bytes)
                }

                override fun onExit(code: Int) {
                    sink.onExit(code)
                }
            }
        val session =
            runtime.value.openLinuxShell(
                projectId,
                rows.toUShort(),
                columns.toUShort(),
                coreSink,
            )
        return CoreLinuxShellHandle(session)
    }
}

private class CoreLinuxShellHandle(
    private val session: app.tether.core.generated.LinuxShellSession,
) : LinuxShellHandle {
    override fun write(bytes: ByteArray) = session.write(bytes)

    override fun resize(
        rows: Int,
        columns: Int,
    ) = session.resize(rows.toUShort(), columns.toUShort())

    /** Kills the shell, then releases the handle even when the kill fails. */
    override fun close() {
        try {
            session.terminate()
        } finally {
            session.close()
        }
    }
}
