package app.tether.session.cards

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import app.tether.core.generated.PermissionModeView
import app.tether.session.agent.AgentLoopException
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.AgentLoopGateway
import app.tether.session.agent.PlanUi
import app.tether.session.agent.QuestionUi
import app.tether.session.agent.TodoUi
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

/** The agent state the cards read. The session screen owns these flows and clears them itself. */
data class CardsSource(
    val todos: StateFlow<List<TodoUi>>,
    val question: StateFlow<QuestionUi?>,
    val plan: StateFlow<PlanUi?>,
)

/** A question the user answered, kept so the card can show the answer after the source clears. */
data class AnsweredQuestion(
    val question: QuestionUi,
    val answer: String,
)

/** The sections of the plan card that can be collapsed. */
enum class PlanSection { Files, Commands, OpenQuestions }

/** Which card a failure belongs to. */
enum class CardKind { Ask, Plan }

/** The state the cards keep for themselves. */
data class CardsUiState(
    val checklistExpanded: Boolean = false,
    val checklistDismissed: Boolean = false,
    val collapsedSections: Set<PlanSection> = emptySet(),
    val runChoiceOpen: Boolean = false,
    val editingPlan: Boolean = false,
    val planDraft: String = "",
    val draft: String = "",
    val answered: AnsweredQuestion? = null,
    val startedPlan: PlanUi? = null,
    val handledPlan: PlanUi? = null,
    val busy: Boolean = false,
    val failure: AgentLoopFailure? = null,
    val failedOn: CardKind? = null,
)

/**
 * What the cards show now. [question] and [answer] are the question on screen and its answer, if
 * the user gave one. [plan] and [planStarted] are the plan on screen and whether it was run.
 */
data class CardsViewState(
    val todos: List<TodoUi>,
    val question: QuestionUi?,
    val answer: String?,
    val plan: PlanUi?,
    val planStarted: Boolean,
    val askFailure: AgentLoopFailure?,
    val planFailure: AgentLoopFailure?,
    val ui: CardsUiState,
)

/**
 * Drives the checklist pin, the question card, and the plan card of one session. The agent state
 * lives in [source]. The cards keep their own record of an answered question and a started plan,
 * because the session clears the source after [onQuestionAnswered] and [onPlanHandled].
 */
class CardsViewModel(
    private val sessionId: String,
    private val gateway: AgentLoopGateway,
    private val source: CardsSource,
    private val onQuestionAnswered: () -> Unit,
    private val onPlanHandled: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val ui = MutableStateFlow(CardsUiState())

    /** What the cards show. */
    val state: StateFlow<CardsViewState> =
        combine(source.todos, source.question, source.plan, ui) { todos, question, plan, current ->
            viewState(todos, question, plan, current)
        }.stateIn(
            viewModelScope,
            SharingStarted.Eagerly,
            viewState(source.todos.value, source.question.value, source.plan.value, ui.value),
        )

    /** Expands or collapses the checklist pin. */
    fun toggleChecklist() {
        ui.update { it.copy(checklistExpanded = !it.checklistExpanded) }
    }

    /** Hides the checklist pin. Only allowed when every item is done. */
    fun dismissChecklist() {
        if (checklistAllDone(source.todos.value)) {
            ui.update { it.copy(checklistDismissed = true) }
        }
    }

    /** Opens or collapses one plan section. */
    fun toggleSection(section: PlanSection) {
        ui.update { current ->
            val collapsed = current.collapsedSections
            current.copy(
                collapsedSections =
                    if (section in
                        collapsed
                    ) {
                        collapsed - section
                    } else {
                        collapsed + section
                    },
            )
        }
    }

    /** Sets the free-text draft of the question card, cut to [ANSWER_MAX_CHARS]. */
    fun setDraft(text: String) {
        ui.update { it.copy(draft = text.take(ANSWER_MAX_CHARS)) }
    }

    /** Answers the question with one of its options. */
    fun answerWithOption(option: String) {
        answer(option)
    }

    /** Sends the free-text draft. A blank draft sends nothing. */
    fun sendDraft() {
        answer(ui.value.draft)
    }

    /** Opens the mode choice of Run this plan. */
    fun openRunChoice() {
        ui.update { it.copy(runChoiceOpen = true) }
    }

    /** Closes the mode choice without running the plan. */
    fun closeRunChoice() {
        ui.update { it.copy(runChoiceOpen = false) }
    }

    /** Runs the plan in [mode]. Plan and Bypass are never passed here. */
    fun runPlan(mode: PermissionModeView) {
        val plan = source.plan.value ?: return
        val current = ui.value
        if (current.busy || plan == current.handledPlan) return
        ui.update { it.copy(busy = true, failure = null, failedOn = null) }
        viewModelScope.launch {
            try {
                withContext(io) { gateway.runPlan(sessionId, mode) }
            } catch (error: AgentLoopException) {
                failWith(error.reason, CardKind.Plan)
                return@launch
            }
            ui.update {
                it.copy(
                    busy = false,
                    startedPlan = plan,
                    handledPlan = plan,
                    runChoiceOpen = false,
                    editingPlan = false,
                )
            }
            onPlanHandled()
        }
    }

    /** Opens the plan text for editing. */
    fun startEditing() {
        val plan = source.plan.value ?: return
        ui.update { it.copy(editingPlan = true, planDraft = planAsText(plan)) }
    }

    /** Sets the text of the plan editor. */
    fun setPlanDraft(text: String) {
        ui.update { it.copy(planDraft = text) }
    }

    /** Leaves the plan editor without sending. */
    fun stopEditing() {
        ui.update { it.copy(editingPlan = false) }
    }

    /** Dismisses the plan card and keeps planning. Fires [onPlanHandled] once per plan. */
    fun keepPlanning() {
        val plan = source.plan.value ?: return
        if (plan == ui.value.handledPlan) return
        ui.update { it.copy(handledPlan = plan, runChoiceOpen = false, editingPlan = false) }
        onPlanHandled()
    }

    private fun answer(text: String) {
        val question = source.question.value
        val clean = answerText(text)
        if (question == null || clean == null || !canAnswer(question)) return
        ui.update { it.copy(busy = true, failure = null, failedOn = null) }
        viewModelScope.launch {
            try {
                withContext(io) { gateway.answerQuestion(sessionId, question.toolCallId, clean) }
            } catch (error: AgentLoopException) {
                failWith(error.reason, CardKind.Ask)
                return@launch
            }
            ui.update {
                it.copy(
                    busy = false,
                    answered = AnsweredQuestion(question, clean),
                    draft = "",
                )
            }
            onQuestionAnswered()
        }
    }

    /** True when no call is running and [question] has no answer yet. */
    private fun canAnswer(question: QuestionUi): Boolean {
        val current = ui.value
        return !current.busy && current.answered?.question != question
    }

    private fun failWith(
        reason: AgentLoopFailure,
        on: CardKind,
    ) {
        ui.update { it.copy(busy = false, failure = reason, failedOn = on) }
    }

    companion object {
        /** Builds the shown state from the source flows and the cards' own record. */
        internal fun viewState(
            todos: List<TodoUi>,
            liveQuestion: QuestionUi?,
            livePlan: PlanUi?,
            current: CardsUiState,
        ): CardsViewState {
            val answered = current.answered
            val question =
                if (answered != null &&
                    (liveQuestion == null || liveQuestion == answered.question)
                ) {
                    answered.question
                } else {
                    liveQuestion
                }
            val answer =
                if (answered != null &&
                    question == answered.question
                ) {
                    answered.answer
                } else {
                    null
                }
            val started = current.startedPlan
            val plan =
                if (started != null &&
                    (livePlan == null || livePlan == started)
                ) {
                    started
                } else {
                    livePlan
                }
            return CardsViewState(
                todos = todos,
                question = question,
                answer = answer,
                plan = plan,
                planStarted = started != null && plan == started,
                askFailure = current.failure.takeIf { current.failedOn == CardKind.Ask },
                planFailure = current.failure.takeIf { current.failedOn == CardKind.Plan },
                ui = current,
            )
        }
    }
}
