package app.tether.memory

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.providers.callCore
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/** What the vault map shows: the core's rendering of the notes around [start], one line each. */
@Immutable
data class VaultMapUiState(
    val loading: Boolean = true,
    val lines: List<String> = emptyList(),
    val problem: AreaProblem? = null,
)

/** State holder for the vault map. The core renders the neighborhood as text. */
class VaultMapViewModel(
    private val gateway: VaultGateway,
    private val start: String,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(VaultMapUiState())

    /** The map state. */
    val state: StateFlow<VaultMapUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            val result = callCore(io) { gateway.map(start, MAP_DEPTH, MAP_BUDGET_BYTES) }
            mutableState.value =
                result.fold(
                    onSuccess = { VaultMapUiState(loading = false, lines = it.lines()) },
                    onFailure = { VaultMapUiState(loading = false, problem = it.areaProblem()) },
                )
        }
    }

    private companion object {
        const val MAP_DEPTH = 2
        const val MAP_BUDGET_BYTES = 4_096
    }
}

/** What the lint screen shows: the notes that no other note links to. */
@Immutable
data class VaultLintUiState(
    val loading: Boolean = true,
    val orphans: List<String> = emptyList(),
    val problem: AreaProblem? = null,
)

/** State holder for lint. Only orphan notes come from the core; no other lint call exists yet. */
class VaultLintViewModel(
    private val gateway: VaultGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(VaultLintUiState())

    /** The lint state. */
    val state: StateFlow<VaultLintUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            val result = callCore(io) { gateway.orphans() }
            mutableState.value =
                result.fold(
                    onSuccess = { VaultLintUiState(loading = false, orphans = it) },
                    onFailure = { VaultLintUiState(loading = false, problem = it.areaProblem()) },
                )
        }
    }
}
