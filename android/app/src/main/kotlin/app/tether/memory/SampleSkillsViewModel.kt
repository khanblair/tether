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
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * What the sample gallery shows. [installing] holds the names being installed right now.
 * [notice] is the sources and licenses text, or null when it could not be read.
 */
@Immutable
data class SampleSkillsUiState(
    val loading: Boolean = true,
    val items: List<SampleSkillItem> = emptyList(),
    val installing: Set<String> = emptySet(),
    val notice: String? = null,
    val problem: AreaProblem? = null,
)

/** State holder for the sample gallery. The core reads and copies files on [io]. */
class SampleSkillsViewModel(
    private val gateway: SkillsGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SampleSkillsUiState())

    /** The gallery state. */
    val state: StateFlow<SampleSkillsUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            val notice = callCore(io) { gateway.samplesNotice() }.getOrNull()
            mutableState.update { it.copy(notice = notice) }
        }
        refresh()
    }

    /** Reads the samples and their install state again. */
    fun refresh() {
        viewModelScope.launch {
            callCore(io) { gateway.samples() }.fold(
                onSuccess = { items ->
                    mutableState.update { it.copy(loading = false, items = items, problem = null) }
                },
                onFailure = { failure ->
                    mutableState.update {
                        it.copy(loading = false, problem = failure.areaProblem())
                    }
                },
            )
        }
    }

    /**
     * Installs the sample [name], then reads the list again. Does nothing for a name that is
     * already installing, already installed, or not in the list.
     */
    fun install(name: String) {
        val state = mutableState.value
        val item = state.items.firstOrNull { it.name == name } ?: return
        if (item.installed || name in state.installing) return
        mutableState.update { it.copy(installing = it.installing + name) }
        viewModelScope.launch {
            val outcome =
                callCore(io) {
                    gateway.installSample(name)
                    gateway.samples()
                }
            outcome.fold(
                onSuccess = { items ->
                    mutableState.update {
                        it.copy(
                            items = items,
                            installing = it.installing - name,
                            problem = null,
                        )
                    }
                },
                onFailure = { failure ->
                    mutableState.update {
                        it.copy(
                            installing = it.installing - name,
                            problem = failure.areaProblem(),
                        )
                    }
                },
            )
        }
    }
}
