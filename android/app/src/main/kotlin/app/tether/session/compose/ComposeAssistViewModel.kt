package app.tether.session.compose

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.files.FilesFailure
import app.tether.files.FilesGateway
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** The gateways and project that the composer's sheets read from. */
data class ComposeAssistDeps(
    val commands: CommandsGateway,
    val files: FilesGateway,
    val projectId: String,
)

/**
 * What the composer's sheets show. [commands] holds the full command list once [commandsLoaded]
 * is true. [mentionQuery] is the query the file results belong to, and [mentionPaths] holds at
 * most [ComposeAssistViewModel.MENTION_LIMIT] of them.
 */
data class ComposeAssistState(
    val mode: AssistMode = AssistMode.None,
    val commands: List<ComposerCommand> = emptyList(),
    val commandsLoaded: Boolean = false,
    val commandsFailed: Boolean = false,
    val mentionQuery: String? = null,
    val mentionPaths: List<String> = emptyList(),
    val mentionLoading: Boolean = false,
    val mentionFailed: Boolean = false,
)

/**
 * Owns the composer's sheet state. The screen calls [onTextChanged] with each text and cursor.
 * File search waits [debounceMs] after the last keystroke, and a result for an older query is
 * dropped. A pick returns the new text, and the sheet stays closed until the text changes again.
 */
class ComposeAssistViewModel(
    private val commands: CommandsGateway,
    private val files: FilesGateway,
    private val projectId: String,
    private val io: CoroutineDispatcher = Dispatchers.IO,
    private val debounceMs: Long = MENTION_DEBOUNCE_MS,
) : ViewModel() {
    private val mutableState = MutableStateFlow(ComposeAssistState())

    /** The sheet state. */
    val state: StateFlow<ComposeAssistState> = mutableState.asStateFlow()

    private var searchJob: Job? = null
    private var searchGeneration = 0

    /** The text a pick just wrote. While the text stays this, no sheet opens. */
    private var suppressedText: String? = null

    init {
        viewModelScope.launch {
            val loaded = loadCommands()
            mutableState.update {
                it.copy(
                    commands = loaded.orEmpty(),
                    commandsLoaded = true,
                    commandsFailed = loaded == null,
                )
            }
        }
    }

    /** The command list, or null when the core cannot list it. */
    @Suppress("TooGenericExceptionCaught")
    private suspend fun loadCommands(): List<ComposerCommand>? =
        try {
            withContext(io) { commands.commands() }
        } catch (_: Exception) {
            null
        }

    /** The composer's text or cursor changed. Opens, moves, or closes the sheet to match. */
    fun onTextChanged(
        text: String,
        cursor: Int,
    ) {
        if (text != suppressedText) suppressedText = null
        val mode = if (suppressedText != null) AssistMode.None else assist(text, cursor)
        mutableState.update { it.copy(mode = mode) }
        when (mode) {
            is AssistMode.Mention -> searchMentions(mode.query)
            is AssistMode.Command, AssistMode.None -> cancelSearch()
        }
    }

    /**
     * The user picked [command]. Returns the new text, or null when the command is not available
     * or the text is not typing a command.
     */
    fun pickCommand(
        command: ComposerCommand,
        text: String,
        cursor: Int,
    ): ComposerEdit? {
        if (!command.available || assist(text, cursor) !is AssistMode.Command) return null
        return applyPick(applyCommand(text, command.name, command.takesArguments))
    }

    /**
     * The user picked the file at [path]. Returns the new text, or null when no mention is open.
     */
    fun pickMention(
        path: String,
        text: String,
        cursor: Int,
    ): ComposerEdit? {
        val mention = assist(text, cursor) as? AssistMode.Mention ?: return null
        return applyPick(applyMention(text, mention, path))
    }

    private fun applyPick(edit: ComposerEdit): ComposerEdit {
        suppressedText = edit.text
        onTextChanged(edit.text, edit.cursor)
        return edit
    }

    private fun searchMentions(query: String) {
        if (mutableState.value.mentionQuery == query) return
        cancelSearch()
        searchGeneration += 1
        val generation = searchGeneration
        mutableState.update {
            it.copy(
                mentionQuery = query,
                mentionPaths = emptyList(),
                mentionLoading = true,
                mentionFailed = false,
            )
        }
        searchJob =
            viewModelScope.launch {
                delay(debounceMs)
                val result = runSearch(query)
                if (generation != searchGeneration) return@launch
                mutableState.update { state ->
                    state.copy(
                        mentionLoading = false,
                        mentionPaths = result.orEmpty().take(MENTION_LIMIT),
                        mentionFailed = result == null,
                    )
                }
            }
    }

    /** Runs the file search, or returns null when the core cannot search. */
    private suspend fun runSearch(query: String): List<String>? =
        try {
            withContext(io) { files.searchFileNames(projectId, query, MENTION_LIMIT) }
        } catch (_: FilesFailure) {
            null
        }

    private fun cancelSearch() {
        searchJob?.cancel()
        searchJob = null
        searchGeneration += 1
        mutableState.update {
            it.copy(
                mentionQuery = null,
                mentionPaths = emptyList(),
                mentionLoading = false,
                mentionFailed = false,
            )
        }
    }

    companion object {
        /** How long the file search waits after the last keystroke, in milliseconds. */
        const val MENTION_DEBOUNCE_MS: Long = 150

        /** The most file results one search shows. */
        const val MENTION_LIMIT: Int = 20

        /** The view model for the composer of the project in [deps]. */
        fun factory(deps: ComposeAssistDeps) =
            viewModelFactory {
                initializer {
                    ComposeAssistViewModel(deps.commands, deps.files, deps.projectId)
                }
            }
    }
}
