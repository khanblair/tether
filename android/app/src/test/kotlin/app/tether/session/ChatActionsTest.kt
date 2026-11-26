package app.tether.session

import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ChatActionsTest {
    private val rewinder = FakeSessionRewinder()
    private val sent = mutableListOf<String>()

    /** How many message rewinds had run when each send ran. A send before its rewind shows 0. */
    private val rewindsAtSend = mutableListOf<Int>()
    private var busy = false
    private var starts = 0
    private var failures = 0

    private fun TestScope.actions(): ChatActions =
        ChatActions(
            scope = this,
            rewinder = rewinder,
            send = { text ->
                rewindsAtSend += rewinder.rewoundToMessages.size
                sent += text
            },
            busy = { busy },
            hooks = ChatTurnHooks(onStart = { starts++ }, onFailed = { failures++ }),
            io = StandardTestDispatcher(testScheduler),
        )

    private fun message(
        id: String,
        role: RoleView,
        text: String,
    ) = MessageView(
        id = id,
        sessionId = "s1",
        role = role,
        seq = 1u,
        createdAtMs = 0L,
        rewound = false,
        parts = listOf(MessagePartView.Text(text)),
    )

    @Test
    fun editRewindsToTheMessageThenSendsTheTrimmedText() =
        runTest {
            val actions = actions()

            assertTrue(actions.edit(message("u1", RoleView.USER, "old"), "  new text "))
            advanceUntilIdle()

            assertEquals(listOf("u1"), rewinder.rewoundToMessages)
            assertEquals(listOf("new text"), sent)
            assertEquals(listOf(1), rewindsAtSend)
            assertEquals(1, starts)
            assertEquals(0, failures)
        }

    @Test
    fun aFailedRewindSendsNothingAndReportsFailure() =
        runTest {
            val actions = actions()
            rewinder.failNextRewindToMessage = SessionSettingsException(SessionFailure.BUSY)

            assertTrue(actions.edit(message("u1", RoleView.USER, "old"), "new"))
            advanceUntilIdle()

            assertEquals(listOf("u1"), rewinder.rewoundToMessages)
            assertTrue(sent.isEmpty())
            assertEquals(1, failures)
        }

    @Test
    fun aFailedSendReportsFailure() =
        runTest {
            val actions =
                ChatActions(
                    scope = this,
                    rewinder = rewinder,
                    send = { throw IllegalStateException("offline") },
                    busy = { false },
                    hooks = ChatTurnHooks(onStart = { starts++ }, onFailed = { failures++ }),
                    io = StandardTestDispatcher(testScheduler),
                )

            assertTrue(actions.edit(message("u1", RoleView.USER, "old"), "new"))
            advanceUntilIdle()

            assertEquals(1, failures)
        }

    @Test
    fun blankEditedTextIsRefusedWithoutRewinding() =
        runTest {
            val actions = actions()

            assertFalse(actions.edit(message("u1", RoleView.USER, "old"), "   "))
            advanceUntilIdle()

            assertTrue(rewinder.rewoundToMessages.isEmpty())
            assertTrue(sent.isEmpty())
            assertEquals(0, starts)
        }

    @Test
    fun regenerateUsesTheNewestUserMessageNotAnEarlierOne() =
        runTest {
            val actions = actions()
            val messages =
                listOf(
                    message("u1", RoleView.USER, "first"),
                    message("a1", RoleView.ASSISTANT, "reply one"),
                    message("u2", RoleView.USER, "second"),
                    message("a2", RoleView.ASSISTANT, "reply two"),
                )

            assertTrue(actions.regenerate(messages))
            advanceUntilIdle()

            assertEquals(listOf("u2"), rewinder.rewoundToMessages)
            assertEquals(listOf("second"), sent)
            assertEquals(listOf(1), rewindsAtSend)
        }

    @Test
    fun regenerateWithNoUserMessageDoesNothing() =
        runTest {
            val actions = actions()

            assertFalse(actions.regenerate(listOf(message("a1", RoleView.ASSISTANT, "hi"))))
            advanceUntilIdle()

            assertTrue(rewinder.rewoundToMessages.isEmpty())
            assertTrue(sent.isEmpty())
        }

    @Test
    fun bothActionsRefuseWhileATurnRunsOrAnApprovalWaits() =
        runTest {
            val actions = actions()
            val messages = listOf(message("u1", RoleView.USER, "hello"))
            busy = true

            assertFalse(actions.edit(messages.first(), "new"))
            assertFalse(actions.regenerate(messages))
            advanceUntilIdle()

            assertTrue(rewinder.rewoundToMessages.isEmpty())
            assertTrue(sent.isEmpty())
            assertEquals(0, starts)
        }
}
