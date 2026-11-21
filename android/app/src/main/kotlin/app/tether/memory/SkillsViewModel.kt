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

/**
 * What the Skills list shows. [skills] is the list filtered by [query], in the core's order.
 * [total] counts every skill folder, whatever the query.
 */
@Immutable
data class SkillsUiState(
    val query: String = "",
    val skills: List<SkillItem> = emptyList(),
    val total: Int = 0,
    val loading: Boolean = true,
    val problem: AreaProblem? = null,
)

/** Keeps the skills whose folder, name, or description contains [query], ignoring case. */
fun filterSkills(
    skills: List<SkillItem>,
    query: String,
): List<SkillItem> {
    val needle = query.trim()
    if (needle.isEmpty()) return skills
    return skills.filter { skill ->
        listOf(
            skill.folder,
            skill.name,
            skill.description,
        ).any { it.contains(needle, ignoreCase = true) }
    }
}

/** State holder for the Skills list. The core reads the skill folders on [io]. */
class SkillsViewModel(
    private val gateway: SkillsGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private var all: List<SkillItem> = emptyList()
    private val mutableState = MutableStateFlow(SkillsUiState())

    /** The list state. */
    val state: StateFlow<SkillsUiState> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads the skill folders again. */
    fun refresh() {
        viewModelScope.launch {
            val result = callCore(io) { gateway.list() }
            result.fold(
                onSuccess = {
                    all = it
                    publish(problem = null)
                },
                onFailure = { publish(problem = it.areaProblem()) },
            )
        }
    }

    /** Filters the list as the user types. */
    fun onQueryChange(text: String) {
        publish(query = text)
    }

    private fun publish(
        query: String = mutableState.value.query,
        problem: AreaProblem? = mutableState.value.problem,
    ) {
        mutableState.value =
            SkillsUiState(
                query = query,
                skills = filterSkills(all, query),
                total = all.size,
                loading = false,
                problem = problem,
            )
    }
}
