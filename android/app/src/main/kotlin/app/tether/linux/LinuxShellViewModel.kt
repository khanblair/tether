package app.tether.linux

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.core.generated.ExecutorRuntimeException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/** The lifecycle of the interactive shell. */
sealed interface LinuxShellState {
    data object Closed : LinuxShellState

    data object Starting : LinuxShellState

    data object Open : LinuxShellState

    data class Exited(
        val code: Int,
    ) : LinuxShellState

    data object NotReady : LinuxShellState

    data object Busy : LinuxShellState

    data class Failed(
        val name: String,
    ) : LinuxShellState
}

/** A throttled render snapshot. The parser itself is never run on a callback thread. */
data class LinuxShellFrame(
    val lines: List<TerminalLine>,
    val visibleLines: Int,
    val cursorLine: Int,
    val cursorRow: Int,
    val cursorColumn: Int,
    val bracketedPaste: Boolean,
)

/** Owns one shell, its parser, and the buffer that survives screen recreation. */
class LinuxShellViewModel(
    private val gateway: LinuxShellGateway,
    private val io: kotlinx.coroutines.CoroutineDispatcher = Dispatchers.IO,
    private val onShellOpenChanged: (Boolean) -> Unit = {},
    private val onSourceClosed: () -> Unit = {},
) : ViewModel() {
    private val parser = AnsiParser(TerminalBuffer())
    private val output = Channel<OutputEvent>(Channel.UNLIMITED)
    private val mutableState = MutableStateFlow<LinuxShellState>(LinuxShellState.Closed)
    private val mutableFrame = MutableStateFlow(parser.buffer.frame())
    private var handle: LinuxShellHandle? = null
    private val closeScope = CoroutineScope(SupervisorJob() + io)

    val state: StateFlow<LinuxShellState> = mutableState.asStateFlow()
    val frame: StateFlow<LinuxShellFrame> = mutableFrame.asStateFlow()

    init {
        viewModelScope.launch { consumeOutput() }
    }

    fun open(
        projectId: String?,
        rows: Int,
        columns: Int,
    ) {
        if (mutableState.value != LinuxShellState.Closed) return
        mutableState.value = LinuxShellState.Starting
        viewModelScope.launch(io) {
            try {
                val opened =
                    gateway.open(
                        projectId,
                        rows,
                        columns,
                        object : LinuxShellSink {
                            override fun onOutput(bytes: ByteArray) {
                                output.trySend(OutputEvent.Bytes(bytes))
                            }

                            override fun onExit(code: Int) {
                                output.trySend(OutputEvent.Exit(code))
                            }
                        },
                    )
                handle = opened
                mutableState.value = LinuxShellState.Open
                onShellOpenChanged(true)
            } catch (error: CancellationException) {
                throw error
            } catch (error: ExecutorRuntimeException) {
                mutableState.value = error.toOpenState()
            } catch (
                @Suppress("TooGenericExceptionCaught") error: Exception,
            ) {
                mutableState.value = LinuxShellState.Failed(error.javaClass.simpleName)
            }
        }
    }

    fun write(bytes: ByteArray) {
        viewModelScope.launch(io) { runShellCall { handle?.let { it.write(bytes) } } }
    }

    fun resize(
        rows: Int,
        columns: Int,
    ) {
        if (rows < 1 || columns < 1) return
        parser.buffer.resize(rows, columns)
        mutableFrame.value = parser.buffer.frame()
        viewModelScope.launch(io) { runShellCall { handle?.let { it.resize(rows, columns) } } }
    }

    fun clear() {
        parser.reset()
        parser.buffer.clear()
        mutableFrame.value = parser.buffer.frame()
    }

    fun closeShell() {
        viewModelScope.launch(io) {
            runShellCall { handle?.let { it.close() } }
            handle = null
            mutableState.value = LinuxShellState.Closed
            onShellOpenChanged(false)
        }
    }

    override fun onCleared() {
        onShellOpenChanged(false)
        onSourceClosed()
        closeScope.launch { runShellCall { handle?.let { it.close() } } }
        output.close()
        super.onCleared()
    }

    private suspend fun consumeOutput() {
        while (true) {
            val first = output.receiveCatching().getOrNull() ?: return
            consume(first)
            withTimeoutOrNull(FRAME_INTERVAL_MS) {
                while (true) {
                    consume(output.receiveCatching().getOrNull() ?: return@withTimeoutOrNull)
                }
            }
            mutableFrame.value = parser.buffer.frame()
        }
    }

    private fun consume(event: OutputEvent) {
        when (event) {
            is OutputEvent.Bytes -> parser.feed(event.bytes)
            is OutputEvent.Exit -> {
                handle = null
                mutableState.value = LinuxShellState.Exited(event.code)
                onShellOpenChanged(false)
            }
        }
    }

    private suspend fun runShellCall(block: () -> Unit) {
        try {
            block()
        } catch (error: CancellationException) {
            throw error
        } catch (
            @Suppress("TooGenericExceptionCaught") error: Exception,
        ) {
            mutableState.value = LinuxShellState.Failed(error.javaClass.simpleName)
        }
    }

    private sealed interface OutputEvent {
        data class Bytes(
            val bytes: ByteArray,
        ) : OutputEvent

        data class Exit(
            val code: Int,
        ) : OutputEvent
    }

    companion object {
        private const val FRAME_INTERVAL_MS = 33L

        fun factory(
            gateway: LinuxShellGateway,
            onShellOpenChanged: (Boolean) -> Unit = {},
            onSourceClosed: () -> Unit = {},
        ) = viewModelFactory {
            initializer {
                LinuxShellViewModel(
                    gateway = gateway,
                    onShellOpenChanged = onShellOpenChanged,
                    onSourceClosed = onSourceClosed,
                )
            }
        }
    }
}

internal fun TerminalBuffer.frame(): LinuxShellFrame =
    LinuxShellFrame(
        lines = lines(),
        visibleLines = rows,
        cursorLine = lines().size - rows + cursorRow,
        cursorRow = cursorRow,
        cursorColumn = cursorColumn,
        bracketedPaste = bracketedPaste,
    )

/** The state a core error leaves the shell in. Any other core error is a failure with its name. */
private fun ExecutorRuntimeException.toOpenState(): LinuxShellState =
    when (this) {
        is ExecutorRuntimeException.LinuxNotReady -> LinuxShellState.NotReady
        is ExecutorRuntimeException.LinuxShellAlreadyOpen -> LinuxShellState.Busy
        else -> LinuxShellState.Failed(javaClass.simpleName)
    }
