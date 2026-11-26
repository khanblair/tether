package app.tether.home

import app.tether.core.generated.ConnectionStatusView
import app.tether.core.generated.ProviderStatusView
import app.tether.core.generated.ProviderView
import app.tether.design.components.SessionState
import app.tether.providers.FakeProviderGateway
import app.tether.providers.connectionMessage
import app.tether.session.FakeSessionGateway
import app.tether.session.SessionStarter
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
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class ProjectHomeViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val providers = FakeProviderGateway()
    private val sessions = FakeSessionGateway()

    private val projectList =
        listOf(
            ProjectSummary("blog", "blog", "main", 0, 1_000L, running = false),
            ProjectSummary("api", "api", "main", 2, 2_000L, running = false),
        )

    private val sessionList =
        listOf(
            SessionSummary("s1", "First", "blog", SessionState.Done, 1_000L, projectId = "blog"),
            SessionSummary("s2", "Second", "api", SessionState.Done, 2_000L, projectId = "api"),
            SessionSummary("s3", "Third", "blog", SessionState.Failed, 1_000L, projectId = "blog"),
        )

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(projectId: String = "blog"): ProjectHomeViewModel {
        val vm =
            ProjectHomeViewModel(
                projectId = projectId,
                projects = fixedProjects(projectList),
                sessions = fixedSessions(sessionList),
                starter = SessionStarter(providers, sessions),
                providers = providers,
                io = dispatcher,
            )
        dispatcher.scheduler.advanceUntilIdle()
        return vm
    }

    private fun fixedProjects(items: List<ProjectSummary>) =
        object : ProjectsSource {
            override fun projects(): List<ProjectSummary> = items
        }

    private fun fixedSessions(items: List<SessionSummary>) =
        object : SessionsSource {
            override fun sessions(): List<SessionSummary> = items
        }

    @Test
    fun the_project_shows_only_its_own_sessions() {
        val state = viewModel().state.value
        assertEquals("blog", state.project?.id)
        assertEquals(listOf("s1", "s3"), state.sessions.map { it.id })
        assertFalse(state.loading)
    }

    @Test
    fun the_last_session_is_the_newest_one_of_the_project() {
        assertEquals(
            "s1",
            viewModel()
                .state.value.lastSession
                ?.id,
        )
    }

    @Test
    fun an_unknown_project_id_has_no_project_and_no_crash() {
        val state = viewModel("missing").state.value
        assertNull(state.project)
        assertTrue(state.sessions.isEmpty())
        assertFalse(state.loading)
    }

    @Test
    fun start_without_a_key_says_no_key_and_creates_no_session() {
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(StartPhase.NoKey, vm.state.value.start)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun start_without_a_model_list_says_no_models() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(StartPhase.NoModels, vm.state.value.start)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun a_gateway_failure_says_failed() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "model"
        sessions.failNext = IllegalStateException("core")
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(StartPhase.Failed, vm.state.value.start)
    }

    @Test
    fun a_second_tap_while_starting_is_ignored() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "model"
        val vm = viewModel()
        vm.startSession()
        assertEquals(StartPhase.Starting, vm.state.value.start)
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(1, sessions.created.size)
    }

    @Test
    fun a_started_session_is_sent_once_to_open() =
        runTest(dispatcher) {
            providers.keys[ProviderView.ANTHROPIC] = "key"
            sessions.models[ProviderView.ANTHROPIC] = "model"
            sessions.nextSessionId = "fresh"
            val vm = viewModel()
            val opened = async { vm.openedSessions.first() }
            vm.startSession()
            advanceUntilIdle()
            assertEquals("fresh", opened.await())
            assertEquals(StartPhase.Ready, vm.state.value.start)
        }

    @Test
    fun test_connection_on_a_missing_model_list_saves_the_list_and_starts_the_session() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(StartPhase.NoModels, vm.state.value.start)
        // The core saves the model list when the provider accepts the key.
        sessions.models[ProviderView.ANTHROPIC] = "model"

        vm.testConnection()
        dispatcher.scheduler.advanceUntilIdle()

        assertTrue(providers.calls.contains("testConnection"))
        assertEquals(listOf(Triple("blog", ProviderView.ANTHROPIC, "model")), sessions.created)
        assertEquals(StartPhase.Ready, vm.state.value.start)
        assertNull(vm.state.value.testProblem)
    }

    @Test
    fun a_rejected_key_in_the_connection_test_shows_its_failure_and_creates_nothing() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        providers.testResult = ProviderStatusView(ConnectionStatusView.BAD_KEY, 0u, null)
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()

        vm.testConnection()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(StartPhase.TestFailed, vm.state.value.start)
        assertEquals(connectionMessage(ConnectionStatusView.BAD_KEY), vm.state.value.testProblem)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun a_connection_test_that_still_has_no_models_stays_on_test_failed_with_no_message() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        providers.testResult = ProviderStatusView(ConnectionStatusView.UNSET, 0u, null)
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()

        vm.testConnection()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(StartPhase.TestFailed, vm.state.value.start)
        assertNull(vm.state.value.testProblem)
        assertTrue(sessions.created.isEmpty())
    }

    @Test
    fun test_connection_with_no_key_left_says_no_key() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        val vm = viewModel()
        vm.startSession()
        dispatcher.scheduler.advanceUntilIdle()
        providers.keys.clear()

        vm.testConnection()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(StartPhase.NoKey, vm.state.value.start)
        assertFalse(providers.calls.contains("testConnection"))
    }

    @Test
    fun test_connection_is_ignored_when_the_start_is_not_a_model_problem() {
        providers.keys[ProviderView.ANTHROPIC] = "key"
        sessions.models[ProviderView.ANTHROPIC] = "model"
        val vm = viewModel()

        vm.testConnection()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(StartPhase.Ready, vm.state.value.start)
        assertFalse(providers.calls.contains("testConnection"))
        assertTrue(sessions.created.isEmpty())
    }
}
