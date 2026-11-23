package app.tether.session.agent

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelStore
import app.tether.core.generated.CoreEventView
import app.tether.session.SessionConnection
import app.tether.session.approval.ApprovalViewModel
import app.tether.session.cards.CardsSource
import app.tether.session.cards.CardsViewModel
import app.tether.session.notices.NoticesViewModel
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.withContext

/**
 * The agent-loop state of one session screen, with the three ViewModels that act on it. One
 * instance belongs to one connection. [state] holds the whole [AgentLoopState]. The ViewModels
 * read slices of it through their own flows, which [publish] keeps in step with [state].
 * [close] clears the ViewModels and their scopes.
 */
class AgentLoop(
    private val sessionId: String,
    private val gateway: AgentLoopGateway,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) {
    private val own = MutableStateFlow(AgentLoopState())
    private val approvalSlot = MutableStateFlow<List<ApprovalUi>>(emptyList())
    private val todoSlot = MutableStateFlow<List<TodoUi>>(emptyList())
    private val questionSlot = MutableStateFlow<QuestionUi?>(null)
    private val planSlot = MutableStateFlow<PlanUi?>(null)
    private val bannerSlot = MutableStateFlow<List<AgentBanner>>(emptyList())
    private val store = ViewModelStore()

    /** The whole agent-loop state: calls, approvals, question, checklist, plan, and more. */
    val state: StateFlow<AgentLoopState> = own.asStateFlow()

    /** Allows or denies the approvals of this session. */
    val approvals: ApprovalViewModel =
        keep(
            "approvals",
            ApprovalViewModel(
                sessionId = sessionId,
                approvals = approvalSlot,
                gateway = gateway,
                onResolved = { id -> update { it.onApprovalResolved(id) } },
                io = io,
            ),
        )

    /** The checklist pin, the question card, and the plan card. */
    val cards: CardsViewModel =
        keep(
            "cards",
            CardsViewModel(
                sessionId = sessionId,
                gateway = gateway,
                source =
                    CardsSource(
                        todos = todoSlot,
                        question = questionSlot,
                        plan = planSlot,
                    ),
                onQuestionAnswered = { update { it.onQuestionAnswered() } },
                onPlanHandled = { update { it.onPlanHandled() } },
                io = io,
            ),
        )

    /** The budget and network notices, and the interrupted turn. */
    val notices: NoticesViewModel =
        keep(
            "notices",
            NoticesViewModel(
                sessionId = sessionId,
                gateway = gateway,
                banners = bannerSlot,
                onBudgetContinued = { update { it.onBudgetContinued() } },
                onInterruptionHandled = {},
                io = io,
            ),
        )

    /** Applies one core event to the state. */
    fun onEvent(event: CoreEventView) {
        update { it.onEvent(event) }
    }

    /** A new turn started: the calls, approvals, and question of the last turn clear. */
    fun onNewTurn() {
        update { it.onNewTurn() }
    }

    /**
     * The user sent the plan back as a message. The plan card closes and its editor closes with
     * it, so the next plan opens with the normal card.
     */
    fun onPlanSent() {
        cards.stopEditing()
        update { it.onPlanHandled() }
    }

    /**
     * Loads the approvals, question, checklist, and plan that the core holds after a restart. A
     * gateway error leaves the state as it was.
     */
    suspend fun hydrate() {
        val restored =
            try {
                withContext(io) { readRestored() }
            } catch (_: AgentLoopException) {
                return
            }
        update {
            it.hydrate(
                approvals = restored.approvals,
                question = restored.question,
                todos = restored.todos,
                plan = restored.plan,
            )
        }
    }

    /** Clears the ViewModels and cancels their scopes. Call it when the screen leaves. */
    fun close() {
        store.clear()
    }

    private fun readRestored(): Restored =
        Restored(
            approvals = gateway.pendingApprovals(sessionId),
            question = gateway.pendingQuestion(sessionId),
            todos = gateway.todos(sessionId),
            plan = gateway.pendingPlan(sessionId),
        )

    private fun update(transform: (AgentLoopState) -> AgentLoopState) {
        own.update(transform)
        publish(own.value)
    }

    private fun publish(next: AgentLoopState) {
        approvalSlot.value = next.approvals
        todoSlot.value = next.todos
        questionSlot.value = next.question
        planSlot.value = next.plan
        bannerSlot.value = next.banners
    }

    private fun <T : ViewModel> keep(
        key: String,
        model: T,
    ): T {
        store.put(key, model)
        return model
    }

    /** What [hydrate] reads from the core before it applies the result. */
    private class Restored(
        val approvals: List<ApprovalUi>,
        val question: QuestionUi?,
        val todos: List<TodoUi>,
        val plan: PlanUi?,
    )
}

/**
 * The [AgentLoop] of [connection], kept for the composition of one connection. Null when there is
 * no connection. The loop closes when the composition leaves or the connection changes.
 */
@Composable
fun rememberAgentLoop(connection: SessionConnection?): AgentLoop? {
    val loop =
        remember(connection) {
            connection?.let { AgentLoop(sessionId = it.sessionId, gateway = it.agent) }
        }
    DisposableEffect(loop) {
        onDispose { loop?.close() }
    }
    return loop
}
