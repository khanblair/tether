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

/**
 * State of one server's detail screen. [busy] is true while a tool change, refresh, or removal
 * runs. [removed] tells the screen to leave, because the server is gone.
 */
@Immutable
data class McpServerDetailUiState(
    val server: McpServer? = null,
    val loading: Boolean = true,
    val loadProblem: McpProblem? = null,
    val busy: Boolean = false,
    val problem: McpProblem? = null,
    val confirmingRemove: Boolean = false,
    val removed: Boolean = false,
)

/**
 * State holder for one server and its tools. Every core call runs on [io]. After each change the
 * server is read again, so the screen shows the saved tool settings.
 */
class McpServerDetailViewModel(
    private val serverId: String,
    private val gateway: McpGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(McpServerDetailUiState())

    /** The screen state. */
    val state: StateFlow<McpServerDetailUiState> = mutableState.asStateFlow()

    init {
        reload()
    }

    /** Reads this server again from the core. */
    fun reload() {
        viewModelScope.launch { loadServer() }
    }

    /** Turns one tool on or off. Its approval stays as it is. */
    fun setToolEnabled(
        tool: String,
        enabled: Boolean,
    ) {
        val current = findTool(tool) ?: return
        runChange { gateway.setTool(serverId, tool, enabled, current.approval) }
    }

    /** Sets one tool to [approval]. Its enabled flag stays as it is. */
    fun setToolApproval(
        tool: String,
        approval: McpApproval,
    ) {
        val current = findTool(tool) ?: return
        runChange { gateway.setTool(serverId, tool, current.enabled, approval) }
    }

    /** Reads the server's tools again from the server. */
    fun refreshTools() {
        runChange { gateway.refreshTools(serverId) }
    }

    /** The user asked to remove the server. The confirmation sheet opens. */
    fun askRemove() {
        mutableState.update { it.copy(confirmingRemove = true) }
    }

    /** The user backed out of the confirmation. */
    fun cancelRemove() {
        mutableState.update { it.copy(confirmingRemove = false) }
    }

    /** The user confirmed. The server is removed, and [McpServerDetailUiState.removed] is set. */
    fun confirmRemove() {
        if (mutableState.value.busy) return
        mutableState.update { it.copy(busy = true, problem = null) }
        viewModelScope.launch {
            callCore(io) { gateway.remove(serverId) }
                .onSuccess {
                    mutableState.update {
                        it.copy(busy = false, confirmingRemove = false, removed = true)
                    }
                }.onFailure { failure ->
                    mutableState.update {
                        it.copy(
                            busy = false,
                            confirmingRemove = false,
                            problem = failure.mcpProblem(),
                        )
                    }
                }
        }
    }

    /** The user closed the problem banner. */
    fun dismissProblem() {
        mutableState.update { it.copy(problem = null) }
    }

    private fun findTool(name: String): McpTool? =
        state.value.server?.tools?.firstOrNull {
            it.name ==
                name
        }

    private fun runChange(block: () -> Unit) {
        if (mutableState.value.busy) return
        mutableState.update { it.copy(busy = true, problem = null) }
        viewModelScope.launch {
            callCore(io, block)
                .onFailure { failure ->
                    mutableState.update { it.copy(problem = failure.mcpProblem()) }
                }
            loadServer()
            mutableState.update { it.copy(busy = false) }
        }
    }

    private suspend fun loadServer() {
        callCore(io) { gateway.servers() }
            .onSuccess { servers ->
                val found = servers.firstOrNull { it.id == serverId }
                mutableState.update {
                    it.copy(
                        server = found,
                        loading = false,
                        loadProblem = if (found == null) McpProblem.NotFound else null,
                    )
                }
            }.onFailure { failure ->
                mutableState.update {
                    it.copy(loading = false, loadProblem = failure.mcpProblem())
                }
            }
    }

    companion object {
        /** Builds the detail model for the server with [serverId]. */
        fun factory(
            serverId: String,
            gateway: McpGateway,
        ) = viewModelFactory { initializer { McpServerDetailViewModel(serverId, gateway) } }
    }
}
