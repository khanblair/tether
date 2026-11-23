package app.tether.session.agent

import app.tether.core.generated.BudgetScopeView
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.PermissionModeView
import app.tether.core.generated.PlanView
import app.tether.core.generated.TodoItemView
import app.tether.core.generated.TodoStatusView
import app.tether.core.generated.ToolCallStatusView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class AgentLoopTest {
    private val dispatcher = StandardTestDispatcher()
    private val gateway = FakeAgentLoopGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun loop() = AgentLoop(sessionId = "s1", gateway = gateway, io = dispatcher)

    private fun check(block: suspend TestScope.() -> Unit) = runTest(dispatcher) { block() }

    private val plan = PlanView("Fix it", listOf("Edit"), listOf("a.kt"), emptyList(), emptyList())

    @Test
    fun eventsUpdateTheState() {
        val loop = loop()

        loop.onEvent(CoreEventView.ToolProposed("s1", "t1", "run", "npm test"))

        assertEquals(
            listOf(ToolCallUi("t1", "run", "npm test", ToolStatus.Proposed, "", false)),
            loop.state.value.calls,
        )
    }

    @Test
    fun aNewTurnClearsCallsButKeepsTodosAndPlan() {
        val loop = loop()
        loop.onEvent(CoreEventView.ToolProposed("s1", "t1", "run", "npm test"))
        loop.onEvent(
            CoreEventView.TodoUpdated("s1", listOf(TodoItemView("Read", TodoStatusView.PENDING))),
        )
        loop.onEvent(CoreEventView.PlanReady("s1", "p1", plan))

        loop.onNewTurn()

        assertTrue(
            loop.state.value.calls
                .isEmpty(),
        )
        assertEquals(listOf(TodoUi("Read", TodoStatus.Pending)), loop.state.value.todos)
        assertEquals(
            PlanUi("p1", "Fix it", listOf("Edit"), listOf("a.kt"), emptyList(), emptyList()),
            loop.state.value.plan,
        )
    }

    @Test
    fun hydrateRestoresApprovalsQuestionTodosAndPlan() =
        check {
            val loop = loop()
            val approval = ApprovalUi("t1", "run", "npm test")
            gateway.approvals = listOf(approval)
            gateway.question = QuestionUi("q1", "Which?", listOf("A"), false)
            gateway.todos = listOf(TodoUi("Read", TodoStatus.InProgress))
            gateway.plan =
                PlanUi(null, "Fix it", emptyList(), emptyList(), emptyList(), emptyList())

            loop.hydrate()
            advanceUntilIdle()

            val state = loop.state.value
            assertEquals(listOf(approval), state.approvals)
            assertEquals(QuestionUi("q1", "Which?", listOf("A"), false), state.question)
            assertEquals(listOf(TodoUi("Read", TodoStatus.InProgress)), state.todos)
            assertEquals(
                PlanUi(null, "Fix it", emptyList(), emptyList(), emptyList(), emptyList()),
                state.plan,
            )
            assertEquals(ToolStatus.AwaitingApproval, state.calls.single().status)
        }

    @Test
    fun aFailingGatewayLeavesTheStateUnchanged() =
        check {
            val loop = loop()
            loop.onEvent(CoreEventView.ToolProposed("s1", "t1", "run", "npm test"))
            val before = loop.state.value
            gateway.failNext = AgentLoopException(AgentLoopFailure.Unavailable)

            loop.hydrate()
            advanceUntilIdle()

            assertEquals(before, loop.state.value)
        }

    @Test
    fun allowingAnApprovalRemovesIt() =
        check {
            val loop = loop()
            loop.onEvent(CoreEventView.ApprovalRequired("s1", "t1", "run", "npm test"))
            assertEquals(1, loop.state.value.approvals.size)

            loop.approvals.allow("t1")
            advanceUntilIdle()

            assertEquals(
                listOf("approve:s1,t1,false"),
                gateway.log.filter { it.startsWith("approve") },
            )
            assertTrue(
                loop.state.value.approvals
                    .isEmpty(),
            )
        }

    @Test
    fun answeringTheQuestionClearsIt() =
        check {
            val loop = loop()
            loop.onEvent(CoreEventView.QuestionAsked("s1", "q1", "Which?", listOf("A", "B"), false))

            loop.cards.answerWithOption("A")
            advanceUntilIdle()

            assertTrue(gateway.log.contains("answerQuestion:s1,q1,A"))
            assertNull(loop.state.value.question)
        }

    @Test
    fun runningThePlanClearsIt() =
        check {
            val loop = loop()
            loop.onEvent(CoreEventView.PlanReady("s1", "p1", plan))

            loop.cards.runPlan(PermissionModeView.AUTO)
            advanceUntilIdle()

            assertTrue(gateway.log.contains("runPlan:s1,AUTO"))
            assertNull(loop.state.value.plan)
        }

    @Test
    fun continuingPastABudgetStopLiftsIt() =
        check {
            val loop = loop()
            loop.onEvent(CoreEventView.BudgetReached("s1", BudgetScopeView.SESSION, 1uL, 1uL))

            loop.notices.onContinue()
            advanceUntilIdle()

            assertTrue(gateway.log.contains("continuePastBudget:s1"))
            assertTrue(
                loop.state.value.banners
                    .none { it is AgentBanner.BudgetReached },
            )
        }

    @Test
    fun sendingThePlanBackClosesIt() =
        check {
            val loop = loop()
            loop.onEvent(CoreEventView.PlanReady("s1", "p1", plan))

            loop.onPlanSent()

            assertNull(loop.state.value.plan)
        }

    @Test
    fun aFinishedCallKeepsItsStatus() {
        val loop = loop()
        loop.onEvent(CoreEventView.ToolProposed("s1", "t1", "run", "npm test"))
        loop.onEvent(CoreEventView.ToolFinished("s1", "t1", ToolCallStatusView.SUCCEEDED))

        assertEquals(
            ToolStatus.Succeeded,
            loop.state.value.calls
                .single()
                .status,
        )
    }
}
