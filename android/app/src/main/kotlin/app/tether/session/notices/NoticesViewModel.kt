package app.tether.session.notices

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.session.agent.AgentBanner
import app.tether.session.agent.AgentLoopGateway
import app.tether.session.agent.InterruptionUi
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** A call the notices can make. Each one shows a progress state while it runs. */
enum class NoticeAction {
    /** Lets the session go on past a budget stop. */
    Continue,

    /** Resumes the interrupted turn. */
    Resume,

    /** Drops the interrupted turn. */
    Discard,
}

/**
 * What the notices show. [warnings], [reached], and [networkPaused] mirror the banners of the
 * session. [interruption] is the stopped turn, or null when none waits. [running] is the call in
 * progress, and [failed] is the call that failed last.
 */
data class NoticesState(
    val warnings: List<AgentBanner.BudgetWarning> = emptyList(),
    val reached: List<AgentBanner.BudgetReached> = emptyList(),
    val networkPaused: Boolean = false,
    val interruption: InterruptionUi? = null,
    val running: NoticeAction? = null,
    val failed: NoticeAction? = null,
)

/**
 * The notices of one session screen. It loads the interrupted turn once, when it starts. The
 * budget and network notices come from [banners]. [onBudgetContinued] runs after the budget stop
 * is lifted, and [onInterruptionHandled] runs after the interrupted turn is resumed or discarded.
 * A failed call keeps its notice and sets [NoticesState.failed], so the user can try again.
 */
class NoticesViewModel(
    private val sessionId: String,
    private val gateway: AgentLoopGateway,
    banners: StateFlow<List<AgentBanner>>,
    private val onBudgetContinued: () -> Unit,
    private val onInterruptionHandled: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val own = MutableStateFlow(NoticesState())

    /** The notices to show. */
    val state: StateFlow<NoticesState> =
        combine(banners, own) { list, current ->
            current.copy(
                warnings = list.filterIsInstance<AgentBanner.BudgetWarning>(),
                reached = list.filterIsInstance<AgentBanner.BudgetReached>(),
                networkPaused = AgentBanner.NetworkPaused in list,
            )
        }.stateIn(viewModelScope, SharingStarted.Eagerly, NoticesState())

    init {
        viewModelScope.launch { loadInterruption() }
    }

    /** The user chose to go on past the budget stop. */
    fun onContinue() {
        if (own.value.running != null) return
        viewModelScope.launch {
            val done = call(NoticeAction.Continue) { gateway.continuePastBudget(sessionId) }
            if (done) onBudgetContinued()
        }
    }

    /** The user chose to resume the interrupted turn. */
    fun onResume() {
        if (own.value.running != null) return
        viewModelScope.launch {
            val done = call(NoticeAction.Resume) { gateway.resume(sessionId) }
            if (done) clearInterruption()
        }
    }

    /** The user chose to discard the interrupted turn. */
    fun onDiscard() {
        if (own.value.running != null) return
        viewModelScope.launch {
            val done = call(NoticeAction.Discard) { gateway.discardInterrupted(sessionId) }
            if (done) clearInterruption()
        }
    }

    private suspend fun loadInterruption() {
        val found = withContext(io) { readInterruption() }
        own.update { it.copy(interruption = found) }
    }

    /** Null when the read fails, so the block stays hidden and a new turn still works. */
    private fun readInterruption(): InterruptionUi? =
        runCatching { gateway.interruption(sessionId) }.getOrNull()

    private fun clearInterruption() {
        own.update { it.copy(interruption = null) }
        onInterruptionHandled()
    }

    /** Runs one gateway call off the main thread. It returns true when the call succeeded. */
    private suspend fun call(
        action: NoticeAction,
        block: () -> Unit,
    ): Boolean {
        own.update { it.copy(running = action, failed = null) }
        val succeeded = withContext(io) { runCall(block) }
        own.update { it.copy(running = null, failed = if (succeeded) null else action) }
        return succeeded
    }

    /** Runs [block]. A failure returns false, and the caller shows it in [NoticesState.failed]. */
    private fun runCall(block: () -> Unit): Boolean = runCatching(block).isSuccess
}
