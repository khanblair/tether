package app.tether.linux

/** Test gateway that lets unit tests emit callback data without Android or native code. */
class FakeLinuxShellGateway(
    private val openFailure: Exception? = null,
) : LinuxShellGateway {
    private var sink: LinuxShellSink? = null
    val handle = FakeLinuxShellHandle()

    override fun open(
        projectId: String?,
        rows: Int,
        columns: Int,
        sink: LinuxShellSink,
    ): LinuxShellHandle {
        openFailure?.let { throw it }
        this.sink = sink
        return handle
    }

    fun emit(text: String) {
        sink?.onOutput(text.toByteArray())
    }

    fun exit(code: Int) {
        sink?.onExit(code)
    }
}

class FakeLinuxShellHandle : LinuxShellHandle {
    override fun write(bytes: ByteArray) = Unit

    override fun resize(
        rows: Int,
        columns: Int,
    ) = Unit

    override fun close() = Unit
}
