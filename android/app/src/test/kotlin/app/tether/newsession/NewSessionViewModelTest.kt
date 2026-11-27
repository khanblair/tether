package app.tether.newsession

import app.tether.home.ProjectSummary
import app.tether.home.ProjectsSource
import app.tether.session.SessionMode
import app.tether.session.SessionStartGateway
import app.tether.session.SessionStartPreview
import app.tether.session.StartResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class NewSessionViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val projects =
        listOf(
            project("old", 10L),
            project("recent", 20L),
        )

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(
        source: FakeProjectsSource = FakeProjectsSource(projects),
        previews: FakeNewSessionGateway = FakeNewSessionGateway(),
        starter: FakeSessionStartGateway = FakeSessionStartGateway(),
        requested: String? = null,
    ): NewSessionViewModel =
        NewSessionViewModel(source, previews, starter, PendingFirstMessage(), requested, dispatcher)
            .also { dispatcher.scheduler.advanceUntilIdle() }

    @Test
    fun loading_finishes_with_the_most_recent_project_ready() {
        val vm =
            NewSessionViewModel(
                FakeProjectsSource(projects),
                FakeNewSessionGateway(),
                FakeSessionStartGateway(),
                PendingFirstMessage(),
                io = dispatcher,
            )

        assertEquals(NewSessionUiState.Loading, vm.state.value)
        dispatcher.scheduler.advanceUntilIdle()

        val state = vm.state.value as NewSessionUiState.Ready

        assertEquals("recent", state.project.id)
        assertEquals(listOf("recent", "old"), state.projects.map(ProjectSummary::id))
    }

    @Test
    fun a_route_project_id_overrides_the_most_recent_project() {
        val state = viewModel(requested = "old").state.value as NewSessionUiState.Ready

        assertEquals("old", state.project.id)
    }

    @Test
    fun switching_project_reloads_its_preview() {
        val previews = FakeNewSessionGateway()
        val vm = viewModel(previews = previews)

        vm.selectProject("old")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("old", (vm.state.value as NewSessionUiState.Ready).project.id)
        assertEquals(listOf("recent", "old"), previews.calls)
    }

    @Test
    fun no_project_shows_no_project() {
        val state = viewModel(source = FakeProjectsSource(emptyList())).state.value

        assertEquals(NewSessionUiState.NoProject, state)
    }

    @Test
    fun a_preview_failure_is_visible() {
        val gateway = FakeNewSessionGateway().apply { failure = IllegalStateException("read") }

        val state = viewModel(previews = gateway).state.value

        assertEquals(NewSessionUiState.Failed, state)
    }

    @Test
    fun send_starts_once_and_hands_the_text_to_the_pending_holder() =
        runTest(dispatcher) {
            val pending = PendingFirstMessage()
            val starter = FakeSessionStartGateway()
            val vm =
                NewSessionViewModel(
                    FakeProjectsSource(projects),
                    FakeNewSessionGateway(),
                    starter,
                    pending,
                    io = dispatcher,
                )
            advanceUntilIdle()
            val opened = async { vm.openedSessions.first() }

            vm.send("Fix the failing test")
            advanceUntilIdle()

            assertEquals(listOf("recent"), starter.startCalls)
            assertEquals("Fix the failing test", pending.take("session-1"))
            assertEquals("session-1", opened.await())
        }

    @Test
    fun a_second_send_while_starting_is_ignored() {
        val starter = FakeSessionStartGateway()
        val vm = viewModel(starter = starter)

        vm.send("first")
        vm.send("second")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf("recent"), starter.startCalls)
    }

    @Test
    fun no_start_happens_before_send() {
        val starter = FakeSessionStartGateway()

        viewModel(starter = starter)

        assertTrue(starter.startCalls.isEmpty())
    }

    @Test
    fun no_key_is_reported_without_navigation() {
        val starter = FakeSessionStartGateway().apply { result = StartResult.NoKey }
        val pending = PendingFirstMessage()
        val vm =
            NewSessionViewModel(
                FakeProjectsSource(projects),
                FakeNewSessionGateway(),
                starter,
                pending,
                io = dispatcher,
            ).also { dispatcher.scheduler.advanceUntilIdle() }

        vm.send("Start")
        dispatcher.scheduler.advanceUntilIdle()

        val state = vm.state.value as NewSessionUiState.Ready
        assertEquals(NewSessionStartError.NoKey, state.startError)
        assertEquals(listOf("recent"), starter.startCalls)
        assertTrue(pending.take("session-1") == null)
    }

    private companion object {
        fun project(
            id: String,
            lastActivity: Long,
        ) = ProjectSummary(id, id, "main", 0, lastActivity, running = false)
    }
}

private class FakeProjectsSource(
    private val values: List<ProjectSummary>,
) : ProjectsSource {
    override fun projects(): List<ProjectSummary> = values
}

private class FakeNewSessionGateway : NewSessionGateway {
    val calls = mutableListOf<String>()
    var failure: Exception? = null

    override fun preview(projectId: String): NewSessionPreview {
        calls += projectId
        failure?.let { throw it }
        return NewSessionPreview("TETHER.md", 3, 1, 2, "Last work", 1_000L)
    }
}

private class FakeSessionStartGateway : SessionStartGateway {
    val startCalls = mutableListOf<String>()
    var result: StartResult = StartResult.Started("session-1")

    override fun preview(projectId: String) =
        SessionStartPreview(modelId = "model", mode = SessionMode.ACCEPT_EDITS)

    override fun start(projectId: String): StartResult {
        startCalls += projectId
        return result
    }
}
