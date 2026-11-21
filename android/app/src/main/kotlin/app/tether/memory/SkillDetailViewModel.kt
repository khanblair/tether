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

/** What the skill detail screen shows: the parsed skill, or the reason it could not be read. */
@Immutable
data class SkillDetailUiState(
    val loading: Boolean = true,
    val detail: SkillDetail? = null,
    val problem: AreaProblem? = null,
)

/** State holder for one skill. The core reads the file on [io]. */
class SkillDetailViewModel(
    private val gateway: SkillsGateway,
    private val folder: String,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SkillDetailUiState())

    /** The screen state. */
    val state: StateFlow<SkillDetailUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            val result = callCore(io) { gateway.read(folder) }
            mutableState.value =
                result.fold(
                    onSuccess = { SkillDetailUiState(loading = false, detail = it) },
                    onFailure = { SkillDetailUiState(loading = false, problem = it.areaProblem()) },
                )
        }
    }
}
