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
 * One header row in the add form. [value] is a secret, so [toString] leaves it out. [revealed]
 * only changes what the field shows.
 */
data class HeaderDraft(
    val name: String = "",
    val value: String = "",
    val revealed: Boolean = false,
) {
    override fun toString(): String = "HeaderDraft(name=$name, value=<hidden>, revealed=$revealed)"
}

/**
 * State of the add form. The rule getters say what the form allows. [addedId] is set once the
 * server is saved, and then the form is read-only and shows [testResult].
 */
@Immutable
data class AddMcpServerUiState(
    val name: String = "",
    val url: String = "",
    val headers: List<HeaderDraft> = emptyList(),
    val adding: Boolean = false,
    val problem: McpProblem? = null,
    val addedId: String? = null,
    val testing: Boolean = false,
    val testResult: McpTestResult? = null,
) {
    /** The name follows the core's rule. */
    val nameValid: Boolean get() = isValidServerName(name)

    /** The address starts with https:// and holds no user name. */
    val urlValid: Boolean get() = isValidServerUrl(url)

    /** Every header row has a valid name and value. */
    val headersValid: Boolean
        get() = headers.all { isValidHeaderName(it.name) && isValidHeaderValue(it.value) }

    /** Add is possible only for a valid form that is not saved or saving yet. */
    val canAdd: Boolean get() = nameValid && urlValid && headersValid && !adding && addedId == null

    /** A header row can be added while the form is open and under the limit. */
    val canAddHeader: Boolean get() = headers.size < MCP_MAX_HEADERS && addedId == null && !adding

    override fun toString(): String =
        "AddMcpServerUiState(name=$name, url=$url, headers=${headers.size}, adding=$adding, " +
            "problem=$problem, addedId=$addedId, testing=$testing, testResult=$testResult)"
}

/**
 * State holder for the add form. Every core call runs on [io]. A saved server's header values
 * leave the state at once, and the test runs once after the save.
 */
class AddMcpServerViewModel(
    private val gateway: McpGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(AddMcpServerUiState())

    /** The screen state. */
    val state: StateFlow<AddMcpServerUiState> = mutableState.asStateFlow()

    /** The name changed. Ignored once the server is saved or while it saves. */
    fun onNameChange(text: String) = edit { copy(name = text) }

    /** The address changed. */
    fun onUrlChange(text: String) = edit { copy(url = text) }

    /** The name of header row [index] changed. */
    fun onHeaderNameChange(
        index: Int,
        text: String,
    ) = edit { copy(headers = headers.updateAt(index) { it.copy(name = text) }) }

    /** The value of header row [index] changed. */
    fun onHeaderValueChange(
        index: Int,
        text: String,
    ) = edit { copy(headers = headers.updateAt(index) { it.copy(value = text) }) }

    /** Shows or hides the value of header row [index]. This only changes the display. */
    fun toggleReveal(index: Int) =
        edit { copy(headers = headers.updateAt(index) { it.copy(revealed = !it.revealed) }) }

    /** Adds an empty header row, up to the limit. */
    fun addHeader() {
        if (!mutableState.value.canAddHeader) return
        edit { copy(headers = headers + HeaderDraft()) }
    }

    /** Removes header row [index]. */
    fun removeHeader(index: Int) =
        edit { copy(headers = headers.filterIndexed { i, _ -> i != index }) }

    /** The user closed the problem banner. */
    fun dismissProblem() {
        mutableState.update { it.copy(problem = null) }
    }

    /**
     * Saves the server, then clears the header values from the state, then runs one test. A
     * failed save keeps the form as it was, so the person can fix it.
     */
    fun add() {
        val typed = mutableState.value
        if (!typed.canAdd) return
        val headers = typed.headers.map { McpHeader(it.name, it.value) }
        mutableState.update { it.copy(adding = true, problem = null) }
        viewModelScope.launch {
            callCore(io) { gateway.add(typed.name, typed.url, headers) }
                .onSuccess { id -> onSaved(id) }
                .onFailure { failure ->
                    mutableState.update { it.copy(adding = false, problem = failure.mcpProblem()) }
                }
        }
    }

    private suspend fun onSaved(id: String) {
        mutableState.update {
            it.copy(adding = false, addedId = id, headers = emptyList(), testing = true)
        }
        val result =
            callCore(io) { gateway.test(id) }
                .fold(
                    onSuccess = { McpTestResult.Connected(it) },
                    onFailure = { McpTestResult.Failed },
                )
        mutableState.update { it.copy(testing = false, testResult = result) }
    }

    private fun edit(change: AddMcpServerUiState.() -> AddMcpServerUiState) {
        mutableState.update { current ->
            if (current.adding ||
                current.addedId != null
            ) {
                current
            } else {
                current.change().copy(problem = null)
            }
        }
    }

    companion object {
        /** Builds the add form model with [gateway]. */
        fun factory(gateway: McpGateway) =
            viewModelFactory { initializer { AddMcpServerViewModel(gateway) } }
    }
}

private fun List<HeaderDraft>.updateAt(
    index: Int,
    change: (HeaderDraft) -> HeaderDraft,
): List<HeaderDraft> = mapIndexed { i, draft -> if (i == index) change(draft) else draft }
