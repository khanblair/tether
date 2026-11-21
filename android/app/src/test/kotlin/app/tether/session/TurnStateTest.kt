package app.tether.session

import app.tether.core.generated.CoreErrorView
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.ErrorKindView
import app.tether.core.generated.SessionStatusView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TurnStateTest {
    private val error = CoreErrorView(ErrorKindView.RATE_LIMITED, "slow down", true)

    private fun updated(status: SessionStatusView) = CoreEventView.SessionUpdated("s1", status)

    @Test fun sendingMakesTheTurnRunningAndClearsEarlierProblems() {
        val before = TurnState(failure = error, stopped = StoppedTurn(null), sendFailed = true)

        val after = before.onSendStarted()

        assertEquals(TurnState(running = true), after)
    }

    @Test fun aFailedSendStopsTheTurnAndSaysSo() {
        val after = TurnState(running = true).onSendFailed()

        assertFalse(after.running)
        assertTrue(after.sendFailed)
        assertTrue(after.hasFooter)
    }

    @Test fun aFailedTurnKeepsItsErrorAndStopsRunning() {
        val running = TurnState(running = true, requestingSinceMs = 100L)

        val after = running.onEvent(CoreEventView.TurnFailed("s1", error), nowMs = 200L)

        assertFalse(after.running)
        assertNull(after.requestingSinceMs)
        assertEquals(error, after.failure)
    }

    @Test fun aFinishedTurnStopsRunningWithoutAProblem() {
        val after = TurnState(running = true).onEvent(CoreEventView.TurnFinished("s1"), nowMs = 0L)

        assertFalse(after.running)
        assertNull(after.failure)
        assertFalse(after.hasFooter)
    }

    @Test fun anInterruptedTurnRemembersTheToolItWasRunning() {
        val running =
            TurnState(running = true, proposedTool = "run" to "npm test")

        val after = running.onEvent(CoreEventView.TurnInterrupted("s1"), nowMs = 0L)

        assertFalse(after.running)
        assertEquals(StoppedTurn("run" to "npm test"), after.stopped)
    }

    @Test fun anInterruptedTurnWithNoToolHasNoToolToName() {
        val after = TurnState(running = true).onEvent(CoreEventView.TurnInterrupted("s1"), 0L)

        assertEquals(StoppedTurn(null), after.stopped)
    }

    @Test fun aSystemInterruptWhileRunningStopsTheTurn() {
        val after =
            TurnState(running = true).onEvent(updated(SessionStatusView.INTERRUPTED), nowMs = 0L)

        assertFalse(after.running)
        assertEquals(StoppedTurn(null), after.stopped)
    }

    @Test fun aSystemInterruptAfterTheTurnEndedIsIgnored() {
        val idle = TurnState(running = false)

        val after = idle.onEvent(updated(SessionStatusView.INTERRUPTED), nowMs = 0L)

        assertNull(after.stopped)
    }

    @Test fun dismissingTheStopClearsIt() {
        val stopped = TurnState(stopped = StoppedTurn(null))

        assertNull(stopped.onDismissStopped().stopped)
    }

    @Test fun theFirstWaitForATokenKeepsItsStartTime() {
        val first = TurnState().onEvent(updated(SessionStatusView.REQUESTING), nowMs = 1_000L)

        val later = first.onEvent(updated(SessionStatusView.REQUESTING), nowMs = 9_000L)

        assertEquals(1_000L, later.requestingSinceMs)
    }

    @Test fun streamingEndsTheWaitForATokenAndHidesThinking() {
        val waiting = TurnState(requestingSinceMs = 1_000L)

        val streaming = waiting.onEvent(updated(SessionStatusView.STREAMING), nowMs = 9_000L)

        assertNull(streaming.requestingSinceMs)
        assertFalse(streaming.hasFooter)
    }

    @Test fun thinkingShowsNoSecondsBeforeFiveSeconds() {
        assertNull(thinkingSeconds(startedAtMs = 0L, nowMs = 4_999L))
    }

    @Test fun thinkingShowsWholeSecondsFromFiveSeconds() {
        assertEquals(5L, thinkingSeconds(startedAtMs = 0L, nowMs = 5_000L))
        assertEquals(7L, thinkingSeconds(startedAtMs = 0L, nowMs = 7_400L))
    }

    @Test fun thinkingCountsFromItsOwnStart() {
        assertEquals(6L, thinkingSeconds(startedAtMs = 2_000L, nowMs = 8_000L))
        assertNull(thinkingSeconds(startedAtMs = 2_000L, nowMs = 2_000L))
    }
}
