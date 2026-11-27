package app.tether.settings

import androidx.compose.runtime.Immutable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.memory.SkillItem
import app.tether.memory.SkillsGateway
import app.tether.providers.callCore
import app.tether.settings.budgets.BudgetLimits
import app.tether.settings.budgets.BudgetsGateway
import app.tether.settings.github.GithubGateway
import app.tether.settings.github.GithubStatus
import app.tether.settings.mcp.McpGateway
import app.tether.settings.mcp.McpServer
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * The text under one Settings row, as plain values. [None] means no subtitle: the row is still
 * loading, or its read failed. The composable turns each value into words.
 */
sealed interface RowText {
    /** No subtitle. */
    data object None : RowText

    /** The GitHub account is signed in. [login] is null when GitHub gave no login. */
    data class SignedIn(
        val login: String?,
    ) : RowText

    /** No GitHub account is saved. */
    data object NotSignedIn : RowText

    /** Saved MCP servers. Zero means none. */
    data class Servers(
        val count: Int,
    ) : RowText

    /** Installed skill folders, valid or not. Zero means none. */
    data class Skills(
        val count: Int,
    ) : RowText

    /** Neither budget limit is set. */
    data object NoLimits : RowText

    /** The daily limit in USD micros. */
    data class DailyLimit(
        val micros: Long,
    ) : RowText

    /** The session limit in USD micros, when no daily limit is set. */
    data class SessionLimit(
        val micros: Long,
    ) : RowText
}

/** The live subtitles of the Settings rows for GitHub, MCP servers, Skills, and Budgets. */
@Immutable
data class SettingsRowsState(
    val github: RowText = RowText.None,
    val mcp: RowText = RowText.None,
    val skills: RowText = RowText.None,
    val budgets: RowText = RowText.None,
)

/** The GitHub row text for [status]. */
internal fun githubRowText(status: GithubStatus): RowText =
    if (status.signedIn) RowText.SignedIn(status.login) else RowText.NotSignedIn

/** The MCP row text for the saved [servers]. */
internal fun mcpRowText(servers: List<McpServer>): RowText = RowText.Servers(servers.size)

/** The Skills row text for every skill folder in [items]. */
internal fun skillsRowText(items: List<SkillItem>): RowText = RowText.Skills(items.size)

/** The Budgets row text: the daily limit first, then the session limit, then none. */
internal fun budgetsRowText(limits: BudgetLimits): RowText =
    when {
        limits.dailyMicros != null -> RowText.DailyLimit(limits.dailyMicros)
        limits.sessionMicros != null -> RowText.SessionLimit(limits.sessionMicros)
        else -> RowText.NoLimits
    }

/**
 * Reads the four live Settings rows. Each read runs on [io] and is independent, so one slow or
 * failing read does not hold up the others. A failed read leaves [RowText.None]. A refresh keeps
 * the last values until the new ones arrive, so a row does not blink on each resume.
 */
class SettingsRowsViewModel(
    private val github: GithubGateway,
    private val mcp: McpGateway,
    private val skills: SkillsGateway,
    private val budgets: BudgetsGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SettingsRowsState())

    /** The row texts. */
    val state: StateFlow<SettingsRowsState> = mutableState.asStateFlow()

    init {
        refresh()
    }

    /** Reads every row again, for when the Settings list resumes. */
    fun refresh() {
        load({ githubRowText(github.status()) }) { copy(github = it) }
        load({ mcpRowText(mcp.servers()) }) { copy(mcp = it) }
        load({ skillsRowText(skills.list()) }) { copy(skills = it) }
        load({ budgetsRowText(budgets.budgets()) }) { copy(budgets = it) }
    }

    private fun load(
        read: () -> RowText,
        place: SettingsRowsState.(RowText) -> SettingsRowsState,
    ) {
        viewModelScope.launch {
            val text = callCore(io, read).getOrDefault(RowText.None)
            mutableState.update { it.place(text) }
        }
    }
}
