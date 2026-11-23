package app.tether.session.compose

import app.tether.session.compose.CommandEffect.ClearContext
import app.tether.session.compose.CommandEffect.SendPrompt
import app.tether.session.compose.SendDecision.Notice
import app.tether.session.compose.SendDecision.Run
import app.tether.session.compose.SendDecision.SendText
import org.junit.Assert.assertEquals
import org.junit.Test

class CommandDispatcherTest {
    private val words =
        CommandWords(
            unknown = { name, suggestion ->
                if (suggestion ==
                    null
                ) {
                    "unknown $name"
                } else {
                    "unknown $name, did you mean $suggestion"
                }
            },
            notAvailableYet = "not yet",
            failed = "failed",
        )

    /** A gateway that answers each typed input with [answer] and records every input it gets. */
    private class ScriptedGateway(
        private val answer: (String) -> CommandResolution,
    ) : CommandsGateway {
        val inputs = mutableListOf<String>()

        override fun commands(): List<ComposerCommand> = emptyList()

        override fun resolve(input: String): CommandResolution {
            inputs += input
            return answer(input)
        }
    }

    private fun dispatcherAnswering(
        answer: (String) -> CommandResolution,
    ): Pair<CommandDispatcher, ScriptedGateway> {
        val gateway = ScriptedGateway(answer)
        return CommandDispatcher(gateway) to gateway
    }

    @Test fun plainTextIsSentWithoutAskingTheGateway() {
        val (dispatcher, gateway) = dispatcherAnswering { error("not called") }

        assertEquals(SendText("hello there"), dispatcher.decide("hello there", words))
        assertEquals(emptyList<String>(), gateway.inputs)
    }

    @Test fun loneSlashIsPlainText() {
        val (dispatcher, gateway) = dispatcherAnswering { error("not called") }

        assertEquals(SendText("/"), dispatcher.decide("/", words))
        assertEquals(emptyList<String>(), gateway.inputs)
    }

    @Test fun slashFollowedByASpaceIsPlainText() {
        val (dispatcher, gateway) = dispatcherAnswering { error("not called") }

        assertEquals(SendText("/ clear"), dispatcher.decide("/ clear", words))
        assertEquals(emptyList<String>(), gateway.inputs)
    }

    @Test fun slashInTheMiddleOfASentenceIsPlainText() {
        val (dispatcher, gateway) = dispatcherAnswering { error("not called") }

        assertEquals(SendText("use /clear later"), dispatcher.decide("use /clear later", words))
        assertEquals(emptyList<String>(), gateway.inputs)
    }

    @Test fun knownCommandRunsItsEffect() {
        val (dispatcher, gateway) =
            dispatcherAnswering { CommandResolution.Resolved("clear", "", "action", null) }

        assertEquals(Run(ClearContext), dispatcher.decide("/clear", words))
        assertEquals(listOf("/clear"), gateway.inputs)
    }

    @Test fun promptCommandWithArgumentsSendsTheFinishedPrompt() {
        val prompt =
            "Write a commit message for the current changes, then commit them. fix the login typo"
        val (dispatcher, _) =
            dispatcherAnswering {
                CommandResolution.Resolved(
                    "commit",
                    "fix the login typo",
                    "prompt",
                    prompt,
                )
            }

        assertEquals(
            Run(SendPrompt(prompt)),
            dispatcher.decide("/commit fix the login typo", words),
        )
    }

    @Test fun unknownCommandWithASuggestionNamesTheClosestOne() {
        val (dispatcher, _) =
            dispatcherAnswering { CommandResolution.Unknown("clera", listOf("clear", "plan")) }

        assertEquals(
            Notice("unknown clera, did you mean clear"),
            dispatcher.decide("/clera", words),
        )
    }

    @Test fun unknownCommandWithoutASuggestionSaysSo() {
        val (dispatcher, _) = dispatcherAnswering { CommandResolution.Unknown("zzz", emptyList()) }

        assertEquals(Notice("unknown zzz"), dispatcher.decide("/zzz", words))
    }

    @Test fun unavailableCommandShowsTheCoreReason() {
        val reason = "/review needs a second model, which is not set up yet"
        val (dispatcher, _) = dispatcherAnswering { CommandResolution.Unavailable(reason) }

        assertEquals(Notice(reason), dispatcher.decide("/review", words))
    }

    @Test fun unavailableCommandWithoutAReasonFallsBackToNotYet() {
        val (dispatcher, _) = dispatcherAnswering { CommandResolution.Unavailable("") }

        assertEquals(Notice("not yet"), dispatcher.decide("/review", words))
    }

    @Test fun runAndCompactAreNotRunYetAndNeverSent() {
        val (runDispatcher, _) =
            dispatcherAnswering { CommandResolution.Resolved("run", "ls", "run", null) }
        val (compactDispatcher, _) =
            dispatcherAnswering { CommandResolution.Resolved("compact", "", "compact", null) }

        assertEquals(Notice("not yet"), runDispatcher.decide("/run ls", words))
        assertEquals(Notice("not yet"), compactDispatcher.decide("/compact", words))
    }

    @Test fun gatewayThatThrowsGivesTheFailedNotice() {
        val dispatcher = CommandDispatcher(ThrowingGateway())

        assertEquals(Notice("failed"), dispatcher.decide("/clear", words))
    }

    @Test fun plainTextKeepsItsSpacingExactly() {
        val (dispatcher, _) = dispatcherAnswering { error("not called") }

        assertEquals(SendText("  two spaces  "), dispatcher.decide("  two spaces  ", words))
    }

    private class ThrowingGateway : CommandsGateway {
        override fun commands(): List<ComposerCommand> = emptyList()

        override fun resolve(input: String): CommandResolution = error("core down")
    }
}
