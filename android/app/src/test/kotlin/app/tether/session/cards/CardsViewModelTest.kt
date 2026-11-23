package app.tether.session.cards

import app.tether.core.generated.PermissionModeView
import app.tether.session.agent.AgentLoopException
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.FakeAgentLoopGateway
import app.tether.session.agent.PlanUi
import app.tether.session.agent.QuestionUi
import app.tether.session.agent.TodoStatus
import app.tether.session.agent.TodoUi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CardsViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeAgentLoopGateway()
    private val todos = MutableStateFlow<List<TodoUi>>(emptyList())
    private val question = MutableStateFlow<QuestionUi?>(sampleQuestion)
    private val plan = MutableStateFlow<PlanUi?>(samplePlan)
    private var answeredCalls = 0
    private var handledCalls = 0

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun model(): CardsViewModel =
        CardsViewModel(
            sessionId = "s1",
            gateway = gateway,
            source = CardsSource(todos, question, plan),
            onQuestionAnswered = { answeredCalls++ },
            onPlanHandled = { handledCalls++ },
            io = dispatcher,
        )

    @Test
    fun answerWithOptionSendsItAndKeepsTheAnswerAfterTheSourceClears() =
        runTest(dispatcher) {
            val vm = model()
            vm.answerWithOption("Fix the code to match the test")
            advanceUntilIdle()
            assertEquals(
                listOf("answerQuestion:s1,call-1,Fix the code to match the test"),
                gateway.log,
            )
            assertEquals(1, answeredCalls)

            // The session clears the question after the callback. The card keeps the answer.
            question.value = null
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals(sampleQuestion, state.question)
            assertEquals("Fix the code to match the test", state.answer)
        }

    @Test
    fun freeTextIsTrimmedBeforeItIsSent() =
        runTest(dispatcher) {
            val vm = model()
            vm.setDraft("  hi there  ")
            vm.sendDraft()
            advanceUntilIdle()
            assertEquals(listOf("answerQuestion:s1,call-1,hi there"), gateway.log)
            assertEquals("", vm.state.value.ui.draft)
        }

    @Test
    fun freeTextIsCappedAtTheLimit() =
        runTest(dispatcher) {
            val vm = model()
            vm.setDraft("x".repeat(600))
            advanceUntilIdle()
            assertEquals(ANSWER_MAX_CHARS, vm.state.value.ui.draft.length)
            vm.sendDraft()
            advanceUntilIdle()
            assertEquals(
                listOf("answerQuestion:s1,call-1,${"x".repeat(ANSWER_MAX_CHARS)}"),
                gateway.log,
            )
        }

    @Test
    fun blankFreeTextIsNotSent() =
        runTest(dispatcher) {
            val vm = model()
            vm.setDraft("   ")
            vm.sendDraft()
            advanceUntilIdle()
            assertTrue(gateway.log.isEmpty())
            assertEquals(0, answeredCalls)
        }

    @Test
    fun runPlanPassesEachOfferedModeToTheGateway() =
        runTest(dispatcher) {
            for (mode in listOf(
                PermissionModeView.ASK,
                PermissionModeView.ACCEPT_EDITS,
                PermissionModeView.AUTO,
            )) {
                val fake = FakeAgentLoopGateway()
                val vm =
                    CardsViewModel(
                        sessionId = "s1",
                        gateway = fake,
                        source = CardsSource(todos, question, plan),
                        onQuestionAnswered = {},
                        onPlanHandled = {},
                        io = dispatcher,
                    )
                vm.runPlan(mode)
                advanceUntilIdle()
                assertEquals(listOf("runPlan:s1,$mode"), fake.log)
                assertEquals(samplePlan, vm.state.value.plan)
                assertTrue(vm.state.value.planStarted)
            }
        }

    @Test
    fun runPlanShowsStartedAndFiresTheCallbackOnce() =
        runTest(dispatcher) {
            val vm = model()
            vm.openRunChoice()
            vm.runPlan(PermissionModeView.ASK)
            vm.runPlan(PermissionModeView.AUTO)
            advanceUntilIdle()
            assertEquals(listOf("runPlan:s1,ASK"), gateway.log)
            assertEquals(1, handledCalls)
            assertFalse(vm.state.value.ui.runChoiceOpen)
        }

    @Test
    fun answerFailureKeepsTheCardOpenAndSetsFailure() =
        runTest(dispatcher) {
            gateway.failNext = AgentLoopException(AgentLoopFailure.Busy)
            val vm = model()
            vm.answerWithOption("Fix the test to match the docs")
            advanceUntilIdle()
            val state = vm.state.value
            assertNull(state.answer)
            assertEquals(sampleQuestion, state.question)
            assertEquals(AgentLoopFailure.Busy, state.askFailure)
            assertFalse(state.ui.busy)
            assertEquals(0, answeredCalls)

            // The next try goes through and clears the failure.
            vm.answerWithOption("Fix the test to match the docs")
            advanceUntilIdle()
            assertEquals("Fix the test to match the docs", vm.state.value.answer)
            assertNull(vm.state.value.askFailure)
            assertEquals(1, answeredCalls)
        }

    @Test
    fun runFailureKeepsTheCardOpenAndSetsPlanFailure() =
        runTest(dispatcher) {
            gateway.failNext = AgentLoopException(AgentLoopFailure.Unavailable)
            val vm = model()
            vm.runPlan(PermissionModeView.ASK)
            advanceUntilIdle()
            val state = vm.state.value
            assertFalse(state.planStarted)
            assertEquals(AgentLoopFailure.Unavailable, state.planFailure)
            assertNull(state.askFailure)
            assertEquals(0, handledCalls)
        }

    @Test
    fun answerIsSentOnlyOnce() =
        runTest(dispatcher) {
            val vm = model()
            vm.answerWithOption("Fix the code to match the test")
            advanceUntilIdle()
            vm.answerWithOption("Fix the test to match the docs")
            advanceUntilIdle()
            assertEquals(1, gateway.log.count { it.startsWith("answerQuestion") })
            assertEquals(1, answeredCalls)
        }

    @Test
    fun keepPlanningFiresTheCallbackOnceAndCallsNoGateway() =
        runTest(dispatcher) {
            val vm = model()
            vm.keepPlanning()
            vm.keepPlanning()
            advanceUntilIdle()
            assertEquals(1, handledCalls)
            assertTrue(gateway.log.isEmpty())
        }

    @Test
    fun aNewPlanCanBeRunAfterAKeptOne() =
        runTest(dispatcher) {
            val vm = model()
            vm.keepPlanning()
            val next = samplePlan.copy(summary = "A second plan.")
            plan.value = next
            vm.runPlan(PermissionModeView.AUTO)
            advanceUntilIdle()
            assertEquals(listOf("runPlan:s1,AUTO"), gateway.log)
            assertEquals(2, handledCalls)
        }

    @Test
    fun editingStartsFromThePlanText() =
        runTest(dispatcher) {
            val vm = model()
            vm.startEditing()
            advanceUntilIdle()
            assertTrue(vm.state.value.ui.editingPlan)
            assertEquals(planAsText(samplePlan), vm.state.value.ui.planDraft)
            vm.setPlanDraft("Edited plan")
            vm.stopEditing()
            advanceUntilIdle()
            assertFalse(vm.state.value.ui.editingPlan)
            assertEquals(0, handledCalls)
        }

    @Test
    fun checklistTogglesAndOnlyDismissesWhenAllDone() =
        runTest(dispatcher) {
            val vm = model()
            todos.value = listOf(TodoUi("one", TodoStatus.InProgress))
            vm.toggleChecklist()
            advanceUntilIdle()
            assertTrue(vm.state.value.ui.checklistExpanded)
            vm.dismissChecklist()
            advanceUntilIdle()
            assertFalse(vm.state.value.ui.checklistDismissed)

            todos.value = listOf(TodoUi("one", TodoStatus.Completed))
            vm.dismissChecklist()
            advanceUntilIdle()
            assertTrue(vm.state.value.ui.checklistDismissed)
            assertFalse(checklistShown(todos.value, vm.state.value.ui.checklistDismissed))
        }

    @Test
    fun sectionsToggleOpenAndClosed() =
        runTest(dispatcher) {
            val vm = model()
            vm.toggleSection(PlanSection.Files)
            advanceUntilIdle()
            assertEquals(setOf(PlanSection.Files), vm.state.value.ui.collapsedSections)
            vm.toggleSection(PlanSection.Files)
            advanceUntilIdle()
            assertTrue(
                vm.state.value.ui.collapsedSections
                    .isEmpty(),
            )
        }

    @Test
    fun planCardsShowNothingWithoutSourceState() =
        runTest(dispatcher) {
            question.value = null
            plan.value = null
            val vm = model()
            advanceUntilIdle()
            assertNull(vm.state.value.question)
            assertNull(vm.state.value.plan)
            vm.keepPlanning()
            vm.startEditing()
            assertEquals(0, handledCalls)
            assertFalse(vm.state.value.ui.editingPlan)
        }

    companion object {
        private val sampleQuestion =
            QuestionUi(
                toolCallId = "call-1",
                question = "Which should change?",
                options =
                    listOf(
                        "Fix the code to match the test",
                        "Fix the test to match the docs",
                    ),
                allowFreeText = true,
            )

        private val samplePlan =
            PlanUi(
                toolCallId = "call-2",
                summary = "Add a limiter.",
                steps = listOf("Add a bucket.", "Wrap the handler."),
                files = listOf("src/limits.py"),
                commands = listOf("pytest -q"),
                openQuestions = listOf("Per key or per IP?"),
            )
    }
}
