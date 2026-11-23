package app.tether.session.compose

import app.tether.files.FilesFailure
import app.tether.files.FilesProblem
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
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
class ComposeAssistViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val commands = FakeCommandsGateway()
    private val files = FakeFilesGateway()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(): ComposeAssistViewModel =
        ComposeAssistViewModel(
            commands,
            files,
            "project-1",
            io = dispatcher,
            debounceMs = 150,
        ).also {
            advanceUntilIdle()
        }

    @Test
    fun slashOpensTheCommandSheetAndAMatchingSpaceClosesIt() =
        runTest(dispatcher) {
            val vm = model()
            vm.onTextChanged("/", 1)
            assertEquals(AssistMode.Command(""), vm.state.value.mode)
            vm.onTextChanged("/co", 3)
            assertEquals(AssistMode.Command("co"), vm.state.value.mode)
            vm.onTextChanged("/compact ", 9)
            assertEquals(AssistMode.None, vm.state.value.mode)
        }

    @Test
    fun commandListLoadsOnceWithTheCoreCommands() =
        runTest(dispatcher) {
            val vm = model()
            assertTrue(vm.state.value.commandsLoaded)
            assertFalse(vm.state.value.commandsFailed)
            assertEquals(FixtureCommands, vm.state.value.commands)
            vm.onTextChanged("/", 1)
            vm.onTextChanged("/c", 2)
            advanceUntilIdle()
            assertEquals(1, commands.listCalls)
        }

    @Test
    fun failedCommandListIsReportedAsFailed() =
        runTest(dispatcher) {
            commands.failure = IllegalStateException("core down")
            val vm = model()
            assertTrue(vm.state.value.commandsLoaded)
            assertTrue(vm.state.value.commandsFailed)
            assertTrue(
                vm.state.value.commands
                    .isEmpty(),
            )
        }

    @Test
    fun unavailableCommandCannotBePicked() =
        runTest(dispatcher) {
            val vm = model()
            val text = "/rev"
            vm.onTextChanged(text, text.length)
            val review = FixtureCommands.single { it.name == "review" }
            assertNull(vm.pickCommand(review, text, text.length))
            assertEquals(AssistMode.Command("rev"), vm.state.value.mode)
        }

    @Test
    fun pickingACommandWithoutArgumentsReplacesTheTextAndCloses() =
        runTest(dispatcher) {
            val vm = model()
            vm.onTextChanged("/pl", 3)
            val edit = vm.pickCommand(FixtureCommands[1], "/pl", 3)
            assertEquals(ComposerEdit("/plan", 5), edit)
            assertEquals(AssistMode.None, vm.state.value.mode)
            vm.onTextChanged("/plan", 5)
            assertEquals(AssistMode.None, vm.state.value.mode)
        }

    @Test
    fun pickingACommandWithArgumentsLeavesASpace() =
        runTest(dispatcher) {
            val vm = model()
            val edit = vm.pickCommand(FixtureCommands[3], "/co", 3)
            assertEquals(ComposerEdit("/commit ", 8), edit)
        }

    @Test
    fun typingAfterAPickOpensTheSheetAgain() =
        runTest(dispatcher) {
            val vm = model()
            vm.pickCommand(FixtureCommands[1], "/pl", 3)
            vm.onTextChanged("/plans", 6)
            assertEquals(AssistMode.Command("plans"), vm.state.value.mode)
        }

    @Test
    fun pickingACommandOutsideCommandModeReturnsNull() =
        runTest(dispatcher) {
            val vm = model()
            assertNull(vm.pickCommand(FixtureCommands[0], "hello", 5))
        }

    @Test
    fun mentionSearchWaitsForTheDebounce() =
        runTest(dispatcher) {
            files.paths = listOf("src/auth.py")
            val vm = model()
            vm.onTextChanged("@a", 2)
            advanceTimeBy(100)
            vm.onTextChanged("@au", 3)
            advanceTimeBy(100)
            assertTrue(files.searches.isEmpty())
            assertTrue(vm.state.value.mentionLoading)
            advanceUntilIdle()
            assertEquals(listOf("au" to 20), files.searches)
            assertEquals(listOf("src/auth.py"), vm.state.value.mentionPaths)
            assertFalse(vm.state.value.mentionLoading)
        }

    @Test
    fun mentionResultsAreCappedAtTwenty() =
        runTest(dispatcher) {
            files.paths = (1..30).map { "file$it.kt" }
            val vm = model()
            vm.onTextChanged("@f", 2)
            advanceUntilIdle()
            assertEquals(20, vm.state.value.mentionPaths.size)
            assertEquals(
                "file1.kt",
                vm.state.value.mentionPaths
                    .first(),
            )
            assertEquals(listOf("f" to 20), files.searches)
        }

    @Test
    fun staleSearchForAnOlderQueryNeverRuns() =
        runTest(dispatcher) {
            files.paths = listOf("alpha.kt")
            val vm = model()
            vm.onTextChanged("@a", 2)
            advanceTimeBy(50)
            vm.onTextChanged("@ab", 3)
            advanceUntilIdle()
            assertEquals(listOf("ab" to 20), files.searches)
            assertEquals("ab", vm.state.value.mentionQuery)
        }

    @Test
    fun resultsForAQueryThatChangedAreNotShown() =
        runTest(dispatcher) {
            files.paths = listOf("alpha.kt")
            val vm = model()
            vm.onTextChanged("@a", 2)
            advanceUntilIdle()
            assertEquals(listOf("alpha.kt"), vm.state.value.mentionPaths)
            vm.onTextChanged("@a ", 3)
            assertEquals(AssistMode.None, vm.state.value.mode)
            assertNull(vm.state.value.mentionQuery)
            assertTrue(
                vm.state.value.mentionPaths
                    .isEmpty(),
            )
        }

    @Test
    fun failedFileSearchIsReportedAsFailed() =
        runTest(dispatcher) {
            files.failure = FilesFailure(FilesProblem.Unavailable)
            val vm = model()
            vm.onTextChanged("@x", 2)
            advanceUntilIdle()
            assertTrue(vm.state.value.mentionFailed)
            assertFalse(vm.state.value.mentionLoading)
            assertTrue(
                vm.state.value.mentionPaths
                    .isEmpty(),
            )
        }

    @Test
    fun pickingAFileInsertsItsPathAndClosesTheSheet() =
        runTest(dispatcher) {
            files.paths = listOf("src/auth/session.py")
            val vm = model()
            vm.onTextChanged("see @au", 7)
            advanceUntilIdle()
            val edit = vm.pickMention("src/auth/session.py", "see @au", 7)
            assertEquals(ComposerEdit("see @src/auth/session.py ", 25), edit)
            assertEquals(AssistMode.None, vm.state.value.mode)
        }

    @Test
    fun pickingAFileWithSpacesInItsPathQuotesIt() =
        runTest(dispatcher) {
            val vm = model()
            vm.onTextChanged("@", 1)
            val edit = vm.pickMention("docs/my notes.md", "@", 1)
            assertEquals(ComposerEdit("@\"docs/my notes.md\" ", 20), edit)
        }

    @Test
    fun pickingAFileOutsideMentionModeReturnsNull() =
        runTest(dispatcher) {
            val vm = model()
            vm.onTextChanged("plain text", 10)
            assertNull(vm.pickMention("src/a.py", "plain text", 10))
        }

    @Test
    fun aQueryReusesItsResultsWithoutSearchingAgain() =
        runTest(dispatcher) {
            files.paths = listOf("a.kt")
            val vm = model()
            vm.onTextChanged("@a", 2)
            advanceUntilIdle()
            vm.onTextChanged("@a", 2)
            advanceUntilIdle()
            assertEquals(1, files.searches.size)
        }
}
