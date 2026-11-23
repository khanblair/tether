package app.tether.session.compose

import app.tether.core.generated.CommandRuntimeException
import org.junit.Assert.assertEquals
import org.junit.Test

class ComposerAssistTest {
    // Command mode: a slash at the very start, with no space before the cursor.

    @Test fun slashAloneOpensTheCommandSheetWithAnEmptyPrefix() {
        assertEquals(AssistMode.Command(""), assist("/", 1))
    }

    @Test fun slashWithLettersGivesThePrefixAfterTheSlash() {
        assertEquals(AssistMode.Command("co"), assist("/co", 3))
    }

    @Test fun cursorInsideTheCommandWordUsesTheTextBeforeTheCursor() {
        assertEquals(AssistMode.Command("co"), assist("/combo", 3))
    }

    @Test fun aSpaceAfterTheCommandClosesTheSheet() {
        assertEquals(AssistMode.None, assist("/compact now", 12))
    }

    @Test fun aSlashInTheMiddleOfASentenceIsNotACommand() {
        assertEquals(AssistMode.None, assist("hello /co", 9))
    }

    @Test fun leadingSpaceBeforeTheSlashIsNotACommand() {
        assertEquals(AssistMode.None, assist(" /co", 4))
    }

    @Test fun emptyTextOpensNothing() {
        assertEquals(AssistMode.None, assist("", 0))
    }

    @Test fun slashWithCursorBeforeItOpensNothing() {
        assertEquals(AssistMode.None, assist("/", 0))
    }

    @Test fun aNewlineAfterTheSlashClosesTheSheet() {
        assertEquals(AssistMode.None, assist("/a\nb", 4))
    }

    @Test fun cursorPastTheEndIsClampedToTheEnd() {
        assertEquals(AssistMode.Command("co"), assist("/co", 99))
    }

    @Test fun negativeCursorIsClampedToZero() {
        assertEquals(AssistMode.None, assist("/co", -4))
    }

    // Mention mode: an @ at the start of a word, with no space between it and the cursor.

    @Test fun atAloneOpensTheMentionSheetWithAnEmptyQuery() {
        assertEquals(AssistMode.Mention("", 0, 1), assist("@", 1))
    }

    @Test fun atWithLettersGivesTheQuery() {
        assertEquals(AssistMode.Mention("au", 0, 3), assist("@au", 3))
    }

    @Test fun mentionAfterAWordStartsAtTheAt() {
        assertEquals(AssistMode.Mention("au", 4, 7), assist("see @au", 7))
    }

    @Test fun emailAddressIsNotAMention() {
        assertEquals(AssistMode.None, assist("a@b.com", 7))
    }

    @Test fun mentionAfterAnEmailAddressIsStillAMention() {
        assertEquals(AssistMode.Mention("x", 13, 15), assist("mail a@b.com @x", 15))
    }

    @Test fun atInsideAnEmailDoesNotOpenTheSheetEvenAfterASpace() {
        assertEquals(AssistMode.None, assist("x a@b.com", 9))
    }

    @Test fun cursorInsideAMentionTakesTheWordBeforeTheCursor() {
        assertEquals(AssistMode.Mention("a", 0, 3), assist("@ab", 2))
    }

    @Test fun cursorRightAfterTheFirstOfTwoMentionsUsesThatMention() {
        assertEquals(AssistMode.Mention("a", 0, 2), assist("@a @b", 2))
    }

    @Test fun cursorAtTheEndUsesTheLastOfTwoMentions() {
        assertEquals(AssistMode.Mention("b", 3, 5), assist("@a @b", 5))
    }

    @Test fun mentionOnASecondLineOpensTheSheet() {
        assertEquals(AssistMode.Mention("fo", 6, 9), assist("first\n@fo", 9))
    }

    @Test fun spaceAfterAMentionClosesTheSheet() {
        assertEquals(AssistMode.None, assist("@au ", 4))
    }

    @Test fun quotedMentionKeepsSpacesInTheQuery() {
        assertEquals(AssistMode.Mention("src/my fi", 0, 11), assist("@\"src/my fi", 11))
    }

    @Test fun cursorInsideAClosedQuotedMentionReadsToTheClosingQuote() {
        assertEquals(AssistMode.Mention("a ", 0, 6), assist("@\"a b\" z", 4))
    }

    @Test fun cursorAfterAClosedQuotedMentionClosesTheSheet() {
        assertEquals(AssistMode.None, assist("@\"a b\"", 6))
    }

    @Test fun quoteBeforeTheAtIsNotAMention() {
        assertEquals(AssistMode.None, assist("\"@foo", 5))
    }

    // Text edits.

    @Test fun applyCommandWithoutArgumentsReplacesTheTypedNameWithNoSpace() {
        assertEquals(
            ComposerEdit("/compact", 8),
            applyCommand("/co", "compact", takesArguments = false),
        )
    }

    @Test fun applyCommandWithArgumentsAddsATrailingSpace() {
        assertEquals(
            ComposerEdit("/commit ", 8),
            applyCommand("/co", "commit", takesArguments = true),
        )
    }

    @Test fun applyCommandKeepsTheRestOfTheLine() {
        assertEquals(
            ComposerEdit("/compact now", 8),
            applyCommand("/co now", "compact", takesArguments = false),
        )
    }

    @Test fun applyCommandWithArgumentsKeepsTheRestAfterOneSpace() {
        assertEquals(
            ComposerEdit("/commit now", 8),
            applyCommand("/co now", "commit", takesArguments = true),
        )
    }

    @Test fun applyCommandLeavesTextWithoutASlashAlone() {
        assertEquals(ComposerEdit("hi", 2), applyCommand("hi", "compact", takesArguments = false))
    }

    @Test fun applyMentionInsertsThePathWithATrailingSpace() {
        val mention = AssistMode.Mention("au", 0, 3)
        assertEquals(
            ComposerEdit("@src/auth/session.py ", 21),
            applyMention("@au", mention, "src/auth/session.py"),
        )
    }

    @Test fun applyMentionReplacesOnlyTheTypedWordInTheMiddle() {
        val mention = AssistMode.Mention("au", 4, 7)
        assertEquals(
            ComposerEdit("see @src/auth/session.py done", 24),
            applyMention("see @au done", mention, "src/auth/session.py"),
        )
    }

    @Test fun applyMentionDoesNotDoubleTheSpaceBeforeExistingText() {
        val mention = AssistMode.Mention("au", 0, 3)
        assertEquals(
            ComposerEdit("@src/auth/session.py more", 20),
            applyMention("@au more", mention, "src/auth/session.py"),
        )
    }

    @Test fun applyMentionQuotesAPathWithSpaces() {
        val mention = AssistMode.Mention("", 0, 1)
        assertEquals(
            ComposerEdit("@\"docs/my notes.md\" ", 20),
            applyMention("@", mention, "docs/my notes.md"),
        )
    }

    @Test fun quotedPathRoundTripsThroughMentionsIn() {
        val mention = AssistMode.Mention("", 0, 1)
        val edit = applyMention("@", mention, "docs/my notes.md")
        assertEquals(listOf(MentionToken("docs/my notes.md", 0, 19)), mentionsIn(edit.text))
    }

    @Test fun removeMentionDeletesTheMentionAndOneSpaceAfterIt() {
        assertEquals(
            ComposerEdit("see and @src/b.py", 4),
            removeMention("see @src/a.py and @src/b.py", "src/a.py"),
        )
    }

    @Test fun removeMentionAtTheEndTakesTheSpaceBeforeIt() {
        assertEquals(ComposerEdit("x", 1), removeMention("x @a", "a"))
    }

    @Test fun removeMentionOfTheOnlyMentionEmptiesTheText() {
        assertEquals(ComposerEdit("", 0), removeMention("@a", "a"))
    }

    @Test fun removeMentionRemovesEveryCopyOfThePath() {
        assertEquals(ComposerEdit("x y", 2), removeMention("x @a y @a", "a"))
    }

    @Test fun removeMentionLeavesTextAloneWhenThePathIsNotMentioned() {
        assertEquals(ComposerEdit("a@b.com", 7), removeMention("a@b.com", "b.com"))
    }

    @Test fun mentionsInFindsEveryMentionInOrder() {
        assertEquals(
            listOf(MentionToken("a", 0, 2), MentionToken("b", 3, 5)),
            mentionsIn("@a @b"),
        )
    }

    @Test fun mentionsInIgnoresEmailAddresses() {
        assertEquals(emptyList<MentionToken>(), mentionsIn("a@b.com"))
    }

    @Test fun mentionsInReadsAQuotedPathWithSpaces() {
        assertEquals(listOf(MentionToken("a b", 0, 6)), mentionsIn("@\"a b\" x"))
    }

    @Test fun mentionsInSkipsAnUnclosedQuote() {
        assertEquals(emptyList<MentionToken>(), mentionsIn("@\"open"))
    }

    @Test fun mentionsInSkipsALoneAt() {
        assertEquals(emptyList<MentionToken>(), mentionsIn("@"))
    }

    @Test fun commandsMatchByPrefixIgnoringCase() {
        val names = commandsMatching(FixtureCommands, "CO").map(ComposerCommand::name)
        assertEquals(listOf("compact", "commit"), names)
    }

    @Test fun emptyPrefixMatchesEveryCommand() {
        assertEquals(FixtureCommands, commandsMatching(FixtureCommands, ""))
    }

    @Test fun truncateMiddleKeepsShortPathsWhole() {
        assertEquals("src/a.py", truncateMiddle("src/a.py"))
    }

    @Test fun truncateMiddleCutsTheMiddleAndKeepsBothEnds() {
        val long = "a".repeat(30) + "b".repeat(30)
        val short = truncateMiddle(long)
        assertEquals(40, short.length)
        assertEquals("a".repeat(20) + "…" + "b".repeat(19), short)
    }

    @Test fun resolutionMapsAnUnknownCommand() {
        val error = CommandRuntimeException.Unknown("zz", listOf("compact"))
        assertEquals(CommandResolution.Unknown("zz", listOf("compact")), error.toResolution())
    }

    @Test fun resolutionMapsAnUnavailableCommand() {
        val error = CommandRuntimeException.Unavailable("needs a second model")
        assertEquals(CommandResolution.Unavailable("needs a second model"), error.toResolution())
    }
}
