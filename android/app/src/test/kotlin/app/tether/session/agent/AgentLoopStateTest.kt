package app.tether.session.agent

import app.tether.core.generated.BudgetScopeView
import app.tether.core.generated.CoreErrorView
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.ErrorKindView
import app.tether.core.generated.PlanView
import app.tether.core.generated.QualityCountsView
import app.tether.core.generated.QualityFindingRecordView
import app.tether.core.generated.TodoItemView
import app.tether.core.generated.TodoStatusView
import app.tether.core.generated.ToolCallStatusView
import app.tether.core.generated.UsageSnapshotView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AgentLoopStateTest {
    private val error = CoreErrorView(ErrorKindView.RATE_LIMITED, "slow down", true)

    private fun proposed(id: String) = CoreEventView.ToolProposed("s1", id, "run", "npm test")

    private fun approvalFor(id: String) =
        CoreEventView.ApprovalRequired("s1", id, "run", "npm test")

    private fun call(
        id: String,
        status: ToolStatus,
        output: String = "",
        truncated: Boolean = false,
    ) = ToolCallUi(id, "run", "npm test", status, output, truncated)

    private fun approval(id: String) = ApprovalUi(id, "run", "npm test")

    private fun quality(toolCallId: String) =
        QualityUi(toolCallId, info = 0, warn = 0, block = 0, findings = emptyList(), skipped = 0)

    @Test fun toolProposedAddsACallWithProposedStatus() {
        val state = AgentLoopState().onEvent(proposed("t1"))

        assertEquals(listOf(call("t1", ToolStatus.Proposed)), state.calls)
    }

    @Test fun aRepeatedProposalKeepsOneCall() {
        val state = AgentLoopState().onEvent(proposed("t1")).onEvent(proposed("t1"))

        assertEquals(1, state.calls.size)
    }

    @Test fun approvalRequiredMarksTheCallAndAddsOneApproval() {
        val once = AgentLoopState().onEvent(proposed("t1")).onEvent(approvalFor("t1"))
        val twice = once.onEvent(approvalFor("t1"))

        assertEquals(listOf(call("t1", ToolStatus.AwaitingApproval)), once.calls)
        assertEquals(listOf(approval("t1")), once.approvals)
        assertEquals(once, twice)
    }

    @Test fun approvalRequiredAddsAnUnknownCall() {
        val state = AgentLoopState().onEvent(approvalFor("t9"))

        assertEquals(listOf(call("t9", ToolStatus.AwaitingApproval)), state.calls)
    }

    @Test fun approvalThenStartedRunsTheCallAndRemovesTheApproval() {
        val state =
            AgentLoopState()
                .onEvent(proposed("t1"))
                .onEvent(approvalFor("t1"))
                .onEvent(CoreEventView.ToolStarted("s1", "t1"))

        assertEquals(listOf(call("t1", ToolStatus.Running)), state.calls)
        assertTrue(state.approvals.isEmpty())
    }

    @Test fun toolFinishedSetsTheFinalStatusAndRemovesTheApproval() {
        val state =
            AgentLoopState()
                .onEvent(approvalFor("t1"))
                .onEvent(
                    CoreEventView.ToolFinished("s1", "t1", ToolCallStatusView.FAILED),
                )

        assertEquals(listOf(call("t1", ToolStatus.Failed)), state.calls)
        assertTrue(state.approvals.isEmpty())
    }

    @Test fun everyCoreToolStatusMapsToItsUiStatus() {
        val mapped = ToolCallStatusView.entries.map { it.toToolStatus() }

        assertEquals(
            listOf(
                ToolStatus.Proposed,
                ToolStatus.Allowed,
                ToolStatus.AwaitingApproval,
                ToolStatus.Running,
                ToolStatus.Denied,
                ToolStatus.Blocked,
                ToolStatus.Succeeded,
                ToolStatus.Failed,
                ToolStatus.TimedOut,
                ToolStatus.Cancelled,
            ),
            mapped,
        )
    }

    @Test fun toolOutputAppendsEachChunk() {
        val state =
            AgentLoopState()
                .onEvent(proposed("t1"))
                .onEvent(CoreEventView.ToolOutput("s1", "t1", "first "))
                .onEvent(CoreEventView.ToolOutput("s1", "t1", "second"))

        assertEquals(call("t1", ToolStatus.Proposed, output = "first second"), state.calls.single())
    }

    @Test fun outputIsCutToTheNewest20000Characters() {
        val first = "a".repeat(MAX_OUTPUT_CHARS)
        val state =
            AgentLoopState()
                .onEvent(proposed("t1"))
                .onEvent(CoreEventView.ToolOutput("s1", "t1", first))
                .onEvent(CoreEventView.ToolOutput("s1", "t1", "xyz"))

        val output = state.calls.single()
        assertEquals(MAX_OUTPUT_CHARS, output.output.length)
        assertTrue(output.output.endsWith("xyz"))
        assertEquals("a".repeat(MAX_OUTPUT_CHARS - 3), output.output.dropLast(3))
        assertTrue(output.outputTruncated)
    }

    @Test fun outputUnderTheLimitIsNotMarkedTruncated() {
        val state =
            AgentLoopState()
                .onEvent(proposed("t1"))
                .onEvent(CoreEventView.ToolOutput("s1", "t1", "a".repeat(MAX_OUTPUT_CHARS)))

        assertFalse(state.calls.single().outputTruncated)
    }

    @Test fun aCutNeverSplitsASurrogatePair() {
        // The emoji is two chars at indexes 10 and 11. The cut starts at index 11, the low half.
        val chunk = "a".repeat(10) + "😀" + "b".repeat(19_999)
        val state =
            AgentLoopState()
                .onEvent(proposed("t1"))
                .onEvent(CoreEventView.ToolOutput("s1", "t1", chunk))

        val output = state.calls.single().output
        assertEquals("b".repeat(19_999), output)
        assertFalse(output.first().isLowSurrogate())
    }

    @Test fun todoUpdatedReplacesTheList() {
        val first =
            CoreEventView.TodoUpdated("s1", listOf(TodoItemView("Read", TodoStatusView.PENDING)))
        val second =
            CoreEventView.TodoUpdated(
                "s1",
                listOf(TodoItemView("Write", TodoStatusView.IN_PROGRESS)),
            )

        val state = AgentLoopState().onEvent(first).onEvent(second)

        assertEquals(listOf(TodoUi("Write", TodoStatus.InProgress)), state.todos)
    }

    @Test fun questionAskedSetsTheQuestion() {
        val event = CoreEventView.QuestionAsked("s1", "q1", "Which?", listOf("A", "B"), false)

        val state = AgentLoopState().onEvent(event)

        assertEquals(QuestionUi("q1", "Which?", listOf("A", "B"), false), state.question)
    }

    @Test fun planReadySetsThePlanWithItsCallId() {
        val plan = PlanView("Fix it", listOf("Edit"), listOf("a.kt"), listOf("gradle"), emptyList())

        val state = AgentLoopState().onEvent(CoreEventView.PlanReady("s1", "p1", plan))

        assertEquals(
            PlanUi("p1", "Fix it", listOf("Edit"), listOf("a.kt"), listOf("gradle"), emptyList()),
            state.plan,
        )
    }

    @Test fun qualityReportAppendsAndKeepsTheNewest20() {
        var state = AgentLoopState()
        for (index in 0..20) {
            state = state.onEvent(qualityEvent("q$index"))
        }

        assertEquals(MAX_QUALITY_REPORTS, state.quality.size)
        assertEquals("q1", state.quality.first().toolCallId)
        assertEquals("q20", state.quality.last().toolCallId)
    }

    @Test fun qualityReportMapsCountsFindingsAndSeverity() {
        val finding = QualityFindingRecordView("R1", "block", "src/a.kt", 7u, "bug", "fix it")
        val event =
            CoreEventView.QualityReport(
                "s1",
                "t1",
                QualityCountsView(1u, 2u, 3u),
                listOf(finding),
                4u,
            )

        val report = AgentLoopState().onEvent(event).quality.single()

        val expectedFinding =
            QualityFindingUi("R1", FindingSeverity.Block, "src/a.kt", 7, "bug", "fix it")
        assertEquals(QualityUi("t1", 1, 2, 3, listOf(expectedFinding), 4), report)
    }

    @Test fun usageUpdatedReplacesTheUsage() {
        val first = usageEvent(1uL, 2uL, 3uL)
        val second = usageEvent(10uL, 20uL, 30uL)

        val state = AgentLoopState().onEvent(first).onEvent(second)

        assertEquals(UsageUi(10L, 20L, 30L), state.usage)
    }

    @Test fun usageClampsValuesAboveTheLongRange() {
        val state = AgentLoopState().onEvent(usageEvent(ULong.MAX_VALUE, 0uL, 0uL))

        assertEquals(Long.MAX_VALUE, state.usage?.tokensIn)
    }

    @Test fun aSecondWarningOfTheSameScopeReplacesTheFirst() {
        val session80 = budgetWarning(BudgetScopeView.SESSION, 80)
        val session90 = budgetWarning(BudgetScopeView.SESSION, 90)
        val day50 = budgetWarning(BudgetScopeView.DAY, 50)

        val state = AgentLoopState().onEvent(session80).onEvent(day50).onEvent(session90)

        assertEquals(
            listOf(
                AgentBanner.BudgetWarning(BudgetScope.Day, 50, 500L, 1_000L),
                AgentBanner.BudgetWarning(BudgetScope.Session, 90, 900L, 1_000L),
            ),
            state.banners,
        )
    }

    @Test fun aSecondReachedOfTheSameScopeReplacesTheFirst() {
        val first = budgetReached(BudgetScopeView.DAY, 100L)
        val second = budgetReached(BudgetScopeView.DAY, 150L)

        val state = AgentLoopState().onEvent(first).onEvent(second)

        val expected = listOf(AgentBanner.BudgetReached(BudgetScope.Day, 150L, 1_000L))
        assertEquals(expected, state.banners)
    }

    @Test fun networkPausedAddsTheBannerOnce() {
        val pause = CoreEventView.NetworkPaused("s1")

        val state = AgentLoopState().onEvent(pause).onEvent(pause)

        assertEquals(listOf(AgentBanner.NetworkPaused), state.banners)
    }

    @Test fun networkResumedRemovesTheBanner() {
        val state =
            AgentLoopState()
                .onEvent(CoreEventView.NetworkPaused("s1"))
                .onEvent(CoreEventView.NetworkResumed("s1"))

        assertTrue(state.banners.isEmpty())
    }

    @Test fun contextCompactedKeepsOnlyTheLatestNotice() {
        val first = CoreEventView.ContextCompacted("s1", "m1", 1_000u, 400u)
        val second = CoreEventView.ContextCompacted("s1", "m2", 900u, 300u)

        val state = AgentLoopState().onEvent(first).onEvent(second)

        assertEquals(listOf(AgentBanner.Compacted(900, 300)), state.banners)
    }

    @Test fun turnFinishedClearsApprovalsAndQuestion() {
        val busy = busyState()

        val after = busy.onEvent(CoreEventView.TurnFinished("s1"))

        assertTrue(after.approvals.isEmpty())
        assertNull(after.question)
    }

    @Test fun turnFailedClearsApprovalsAndQuestion() {
        val after = busyState().onEvent(CoreEventView.TurnFailed("s1", error))

        assertTrue(after.approvals.isEmpty())
        assertNull(after.question)
    }

    @Test fun turnInterruptedClearsApprovalsAndQuestion() {
        val after = busyState().onEvent(CoreEventView.TurnInterrupted("s1"))

        assertTrue(after.approvals.isEmpty())
        assertNull(after.question)
    }

    @Test fun anUnknownEventLeavesTheStateUnchanged() {
        val state = busyState()

        assertEquals(state, state.onEvent(CoreEventView.TextDelta("s1", "m1", "hi")))
        assertEquals(state, state.onEvent(CoreEventView.Ready("1")))
    }

    @Test fun newTurnKeepsTodosPlanAndUsage() {
        val state =
            AgentLoopState(
                calls = listOf(call("t1", ToolStatus.Succeeded)),
                approvals = listOf(approval("t1")),
                question = QuestionUi("q1", "Which?", emptyList(), true),
                todos = listOf(TodoUi("Read", TodoStatus.Pending)),
                plan = PlanUi("p1", "Plan", emptyList(), emptyList(), emptyList(), emptyList()),
                quality = listOf(quality("t1")),
                banners = listOf(AgentBanner.NetworkPaused, AgentBanner.Compacted(5, 2)),
                usage = UsageUi(1L, 2L, 3L),
            )

        val next = state.onNewTurn()

        assertTrue(next.calls.isEmpty())
        assertTrue(next.approvals.isEmpty())
        assertNull(next.question)
        assertTrue(next.quality.isEmpty())
        assertEquals(listOf(AgentBanner.Compacted(5, 2)), next.banners)
        assertEquals(state.todos, next.todos)
        assertEquals(state.plan, next.plan)
        assertEquals(state.usage, next.usage)
    }

    @Test fun answeringHandlesClearTheirItems() {
        val state =
            busyState()
                .onEvent(CoreEventView.PlanReady("s1", "p1", emptyPlan()))
                .onEvent(CoreEventView.BudgetReached("s1", BudgetScopeView.DAY, 1uL, 1uL))

        val after =
            state
                .onApprovalResolved("t1")
                .onQuestionAnswered()
                .onPlanHandled()
                .onBudgetContinued()

        assertTrue(after.approvals.isEmpty())
        assertNull(after.question)
        assertNull(after.plan)
        assertTrue(after.banners.none { it is AgentBanner.BudgetReached })
    }

    @Test fun hydrateRestoresTheStateAndItsCalls() {
        val restored =
            AgentLoopState().hydrate(
                approvals = listOf(approval("t1"), approval("t1")),
                question = QuestionUi("q1", "Which?", emptyList(), false),
                todos = listOf(TodoUi("Read", TodoStatus.Completed)),
                plan = PlanUi(null, "Plan", emptyList(), emptyList(), emptyList(), emptyList()),
            )

        assertEquals(listOf(approval("t1")), restored.approvals)
        assertEquals(listOf(call("t1", ToolStatus.AwaitingApproval)), restored.calls)
        assertEquals(QuestionUi("q1", "Which?", emptyList(), false), restored.question)
        assertEquals(listOf(TodoUi("Read", TodoStatus.Completed)), restored.todos)
        assertNull(restored.plan?.toolCallId)
    }

    @Test fun hydrateKeepsCallsThatAlreadyExist() {
        val known = AgentLoopState(calls = listOf(call("t1", ToolStatus.Running)))

        val restored = known.hydrate(listOf(approval("t1")), null, emptyList(), null)

        assertEquals(listOf(call("t1", ToolStatus.Running)), restored.calls)
    }

    private fun emptyPlan() = PlanView("S", emptyList(), emptyList(), emptyList(), emptyList())

    private fun busyState(): AgentLoopState =
        AgentLoopState()
            .onEvent(approvalFor("t1"))
            .onEvent(CoreEventView.QuestionAsked("s1", "q1", "Which?", emptyList(), true))

    private fun qualityEvent(id: String) =
        CoreEventView.QualityReport("s1", id, QualityCountsView(0u, 0u, 0u), emptyList(), 0u)

    private fun usageEvent(
        tokensIn: ULong,
        tokensOut: ULong,
        costMicros: ULong,
    ) = CoreEventView.UsageUpdated("s1", UsageSnapshotView(tokensIn, tokensOut, costMicros))

    private fun budgetWarning(
        scope: BudgetScopeView,
        percent: Int,
    ) = CoreEventView.BudgetWarning(
        "s1",
        scope,
        percent.toUByte(),
        (percent * 10L).toULong(),
        1_000uL,
    )

    private fun budgetReached(
        scope: BudgetScopeView,
        spent: Long,
    ) = CoreEventView.BudgetReached("s1", scope, spent.toULong(), 1_000uL)
}
