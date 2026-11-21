package app.tether.session

import app.tether.core.generated.CoreErrorView
import app.tether.core.generated.ErrorKindView
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class TurnProblemTest {
    private fun error(
        kind: ErrorKindView,
        retryable: Boolean = false,
    ) = CoreErrorView(kind = kind, message = "reason", retryable = retryable)

    private fun message(
        id: String,
        role: RoleView,
        vararg parts: MessagePartView,
    ) = MessageView(
        id = id,
        sessionId = "s1",
        role = role,
        seq = 1u,
        createdAtMs = 0L,
        rewound = false,
        parts = parts.toList(),
    )

    @Test fun authErrorMapsToKeyRejected() {
        assertEquals(TurnProblem.KeyRejected, turnProblemOf(error(ErrorKindView.AUTH)))
    }

    @Test fun rateLimitedErrorMapsToRateLimited() {
        assertEquals(TurnProblem.RateLimited, turnProblemOf(error(ErrorKindView.RATE_LIMITED)))
    }

    @Test fun networkErrorMapsToNetwork() {
        assertEquals(TurnProblem.Network, turnProblemOf(error(ErrorKindView.NETWORK)))
    }

    @Test fun internalErrorMapsToOther() {
        assertEquals(TurnProblem.Other, turnProblemOf(error(ErrorKindView.INTERNAL)))
    }

    @Test fun invalidInputErrorMapsToOther() {
        assertEquals(TurnProblem.Other, turnProblemOf(error(ErrorKindView.INVALID_INPUT)))
    }

    @Test fun notFoundErrorMapsToOther() {
        assertEquals(TurnProblem.Other, turnProblemOf(error(ErrorKindView.NOT_FOUND)))
    }

    @Test fun cancelledErrorMapsToOther() {
        assertEquals(TurnProblem.Other, turnProblemOf(error(ErrorKindView.CANCELLED)))
    }

    @Test fun rateLimitOffersRetryEvenWhenTheCoreSaysNotRetryable() {
        val problem = TurnProblem.RateLimited

        assertTrue(problem.offersRetry(error(ErrorKindView.RATE_LIMITED, retryable = false)))
    }

    @Test fun networkOffersRetryOnlyWhenRetryable() {
        assertTrue(TurnProblem.Network.offersRetry(error(ErrorKindView.NETWORK, true)))
        assertFalse(TurnProblem.Network.offersRetry(error(ErrorKindView.NETWORK, false)))
    }

    @Test fun otherOffersRetryOnlyWhenRetryable() {
        assertTrue(TurnProblem.Other.offersRetry(error(ErrorKindView.INTERNAL, true)))
        assertFalse(TurnProblem.Other.offersRetry(error(ErrorKindView.INTERNAL, false)))
    }

    @Test fun keyRejectedNeverOffersRetry() {
        assertFalse(TurnProblem.KeyRejected.offersRetry(error(ErrorKindView.AUTH, true)))
    }

    @Test fun retryResendsTheNewestUserMessage() {
        val messages =
            listOf(
                message("m1", RoleView.USER, MessagePartView.Text("first question")),
                message("m2", RoleView.ASSISTANT, MessagePartView.Text("an answer")),
                message("m3", RoleView.USER, MessagePartView.Text("fix the login test")),
                message("m4", RoleView.ASSISTANT, MessagePartView.Text("the reply that failed")),
            )

        assertEquals("fix the login test", lastUserText(messages))
    }

    @Test fun retryHasNothingToResendWithoutAUserMessage() {
        val messages = listOf(message("m1", RoleView.ASSISTANT, MessagePartView.Text("hello")))

        assertNull(lastUserText(messages))
    }

    @Test fun retryHasNothingToResendWhenTheUserMessageHasNoText() {
        val messages = listOf(message("m1", RoleView.USER, MessagePartView.Thinking("")))

        assertNull(lastUserText(messages))
    }
}
