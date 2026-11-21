package app.tether.home

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** Where the New empty project screen stands. */
enum class NewProjectPhase {
    /** The user is typing. Create is on when the name is usable. */
    Editing,

    /** The core is creating the folder. The field and Create are off. */
    Creating,

    /** The core could not create the project. Nothing was created. */
    Failed,

    /** The project exists. The screen opens its home. */
    Created,
}

/**
 * What the New empty project screen shows. [problem] is the reason under the field, or null when
 * the name is usable or still empty. [projectId] is set once the project exists.
 */
@Immutable
data class NewProjectUiState(
    val name: String = "",
    val phase: NewProjectPhase = NewProjectPhase.Editing,
    val problem: ProjectNameProblem? = null,
    val canCreate: Boolean = false,
    val projectId: String? = null,
)

/**
 * State holder for New empty project. It checks the name as the user types, and creates the
 * project on [io], because the core blocks.
 */
class NewProjectViewModel(
    private val projects: ProjectGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(NewProjectUiState())

    /** The screen state. */
    val state: StateFlow<NewProjectUiState> = mutableState.asStateFlow()

    /** Records the typed name and checks it. Ignored while the project is being created. */
    fun onNameChange(name: String) {
        if (mutableState.value.phase == NewProjectPhase.Creating) return
        mutableState.value = editing(name)
    }

    /** Creates the project named in the field. Does nothing when the name is not usable. */
    fun create() {
        val current = mutableState.value
        if (!current.canCreate || current.phase == NewProjectPhase.Creating) return
        val check = checkProjectName(current.name) as? ProjectNameCheck.Valid ?: return
        mutableState.update { it.copy(phase = NewProjectPhase.Creating) }
        viewModelScope.launch(io) { mutableState.value = result(check.name) }
    }

    private fun editing(name: String): NewProjectUiState {
        val check = checkProjectName(name)
        return NewProjectUiState(
            name = name,
            phase = NewProjectPhase.Editing,
            problem = (check as? ProjectNameCheck.Invalid)?.problem,
            canCreate = check is ProjectNameCheck.Valid,
        )
    }

    @Suppress("TooGenericExceptionCaught")
    private fun result(name: String): NewProjectUiState {
        val outcome =
            try {
                projects.create(name)
            } catch (_: Exception) {
                CreateProjectResult.Unavailable
            }
        return when (outcome) {
            is CreateProjectResult.Created ->
                mutableState.value.copy(
                    phase = NewProjectPhase.Created,
                    projectId = outcome.project.id,
                )
            CreateProjectResult.InvalidName,
            CreateProjectResult.Unavailable,
            -> mutableState.value.copy(phase = NewProjectPhase.Failed)
        }
    }

    companion object {
        /** Builds the screen model from the project gateway. */
        fun factory(projects: ProjectGateway) =
            viewModelFactory {
                initializer { NewProjectViewModel(projects) }
            }
    }
}
