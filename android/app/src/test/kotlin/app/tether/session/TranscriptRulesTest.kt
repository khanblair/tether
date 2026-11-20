package app.tether.session

import app.tether.core.generated.CoreErrorView
import app.tether.core.generated.CoreEventView
import app.tether.core.generated.ErrorKindView
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TranscriptRulesTest {
    @Test fun turnFinishedEndsTheTurn() {
        assertTrue(CoreEventView.TurnFinished("s1").endsTurn())
    }

    @Test fun turnFailedEndsTheTurn() {
        val error = CoreErrorView(ErrorKindView.INTERNAL, "failed", false)
        assertTrue(CoreEventView.TurnFailed("s1", error).endsTurn())
    }

    @Test fun turnInterruptedEndsTheTurn() {
        assertTrue(CoreEventView.TurnInterrupted("s1").endsTurn())
    }

    @Test fun textDeltaDoesNotEndTheTurn() {
        assertFalse(CoreEventView.TextDelta("s1", "m1", "hello").endsTurn())
    }

    @Test fun networkResumedDoesNotEndTheTurn() {
        assertFalse(CoreEventView.NetworkResumed("s1").endsTurn())
    }

    @Test fun readerAtBottomFollowsNewContent() {
        assertTrue(shouldFollow(wasAtBottom = true, hasNewContent = true))
    }

    @Test fun readerScrolledUpDoesNotFollow() {
        assertFalse(shouldFollow(wasAtBottom = false, hasNewContent = true))
    }

    @Test fun noNewContentDoesNotFollow() {
        assertFalse(shouldFollow(wasAtBottom = true, hasNewContent = false))
    }
}
