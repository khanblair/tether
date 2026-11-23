package app.tether.settings.mcp

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** State of the MCP server list. [testing] holds the ids with a test in flight. */
@Immutable
data class McpServersUiState(
    val servers: List<McpServer> = emptyList(),
    val loading: Boolean = true,
    val loadFailed: Boolean = false,
    val actionProblem: McpProblem? = null,
    val testing: Set<String> = emptySet(),
    val testResults: Map<String, McpTestResult> = emptyMap(),
)

/**
 * State holder for the server list. Every core call runs on [io]. After each change the list is
 * read again, so the screen always shows what the core saved.
 */
class McpServersViewModel(
    private val gateway: McpGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(McpServersUiState())

    /** The screen state. */
    val state: StateFlow<McpServersUiState> = mutableState.asStateFlow()

    init {
        reload()
    }

    /** Reads the server list again. The screen calls this after the add sheet closes. */
    fun reload() {
        viewModelScope.launch { loadServers() }
    }

    /** Turns [id] on or off. The list is read again afterwards. */
    fun setEnabled(
        id: String,
        enabled: Boolean,
    ) {
        runChange { gateway.setEnabled(id, enabled) }
    }

    /** Connects to [id] once and keeps the result under its row. Ignored while that test runs. */
    fun test(id: String) {
        if (id in state.value.testing) return
        mutableState.update { it.copy(testing = it.testing + id) }
        viewModelScope.launch {
            val result =
                callCore(io) { gateway.test(id) }
                    .fold(
                        onSuccess = { McpTestResult.Connected(it) },
                        onFailure = { McpTestResult.Failed },
                    )
            mutableState.update {
                it.copy(testing = it.testing - id, testResults = it.testResults + (id to result))
            }
            loadServers()
        }
    }

    /** The user closed the problem banner. */
    fun dismissProblem() {
        mutableState.update { it.copy(actionProblem = null) }
    }

    private fun runChange(block: () -> Unit) {
        mutableState.update { it.copy(actionProblem = null) }
        viewModelScope.launch {
            callCore(io, block)
                .onSuccess { loadServers() }
                .onFailure { failure ->
                    mutableState.update { it.copy(actionProblem = failure.mcpProblem()) }
                }
        }
    }

    private suspend fun loadServers() {
        callCore(io) { gateway.servers() }
            .onSuccess { servers ->
                mutableState.update {
                    it.copy(servers = servers, loading = false, loadFailed = false)
                }
            }.onFailure {
                mutableState.update { it.copy(loading = false, loadFailed = true) }
            }
    }

    companion object {
        /** Builds the list model with [gateway]. */
        fun factory(gateway: McpGateway) =
            viewModelFactory { initializer { McpServersViewModel(gateway) } }
    }
}
