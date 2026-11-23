package app.tether.settings.github

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

/** The identity form. [saved] turns true once the core stores the identity. */
data class IdentityUiState(
    val name: String = "",
    val email: String = "",
    val nameProblem: IdentityProblem? = null,
    val emailProblem: IdentityProblem? = null,
    val saving: Boolean = false,
    val saved: Boolean = false,
    val failure: GithubFailure? = null,
)

/**
 * State holder for the Git identity form. When the account is signed in, the empty fields start
 * with the GitHub name and email. Save checks both fields before it calls the core.
 */
class GitIdentityViewModel(
    private val gateway: GithubGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(IdentityUiState())

    /** The screen state. */
    val state: StateFlow<IdentityUiState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            callCore(io) { gateway.status() }
                .onSuccess { status ->
                    if (status.signedIn) prefill(status)
                }
        }
    }

    /** The user typed in the name field. */
    fun onNameChange(value: String) {
        mutableState.update { it.copy(name = value, nameProblem = null, saved = false) }
    }

    /** The user typed in the email field. */
    fun onEmailChange(value: String) {
        mutableState.update { it.copy(email = value, emailProblem = null, saved = false) }
    }

    /** The user pressed Save. Nothing is saved while a field is invalid. */
    fun save() {
        val current = mutableState.value
        val nameProblem = identityProblem(current.name)
        val emailProblem = identityProblem(current.email)
        if (nameProblem != null || emailProblem != null) {
            mutableState.update { it.copy(nameProblem = nameProblem, emailProblem = emailProblem) }
            return
        }
        mutableState.update { it.copy(saving = true, failure = null) }
        viewModelScope.launch {
            callCore(io) { gateway.setIdentity(current.name.trim(), current.email.trim()) }
                .onSuccess {
                    mutableState.update { it.copy(saving = false, saved = true) }
                }.onFailure { error ->
                    mutableState.update {
                        it.copy(saving = false, failure = error.githubFailure())
                    }
                }
        }
    }

    private fun prefill(status: GithubStatus) {
        mutableState.update { state ->
            state.copy(
                name = if (state.name.isEmpty()) status.name.orEmpty() else state.name,
                email = if (state.email.isEmpty()) status.email.orEmpty() else state.email,
            )
        }
    }
}
