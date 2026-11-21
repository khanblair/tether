package app.tether.session

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
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class SearchViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val settings = FakeSessionSettingsGateway()
    private val hit = SearchHit("s1", "Fix login", "token expires at the boundary")

    @Before
    fun setUp() {
        Dispatchers.setMain(dispatcher)
        settings.transcriptHits = listOf(hit)
    }

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun TestScope.model(
        projectId: String? = null,
        initialQuery: String = "",
    ): SearchViewModel {
        val vm = SearchViewModel(settings, projectId, initialQuery, dispatcher)
        advanceUntilIdle()
        return vm
    }

    private fun searchCalls(): List<String> = settings.calls.filter { it.startsWith("search") }

    @Test
    fun a_blank_query_never_reaches_the_core() =
        runTest(dispatcher) {
            val vm = model()
            vm.onQueryChange("   ")
            advanceUntilIdle()
            assertTrue(searchCalls().isEmpty())
            assertFalse(vm.state.value.searched)
        }

    @Test
    fun typing_waits_300_ms_before_the_search_runs() =
        runTest(dispatcher) {
            val vm = model()
            vm.onQueryChange("boundary")
            advanceTimeBy(299)
            assertTrue(searchCalls().isEmpty())
            advanceTimeBy(1)
            advanceUntilIdle()
            assertEquals(listOf("searchTranscripts:boundary:50"), searchCalls())
            assertEquals(listOf(hit), vm.state.value.hits)
        }

    @Test
    fun each_keystroke_restarts_the_wait_so_only_the_last_query_runs() =
        runTest(dispatcher) {
            val vm = model()
            vm.onQueryChange("bou")
            advanceTimeBy(200)
            vm.onQueryChange("boundary")
            advanceUntilIdle()
            assertEquals(listOf("searchTranscripts:boundary:50"), searchCalls())
        }

    @Test
    fun a_query_with_no_hits_is_searched_and_empty() =
        runTest(dispatcher) {
            settings.transcriptHits = emptyList()
            val vm = model()
            vm.onQueryChange("nothing here")
            advanceUntilIdle()
            val state = vm.state.value
            assertTrue(state.searched)
            assertTrue(state.hits.isEmpty())
        }

    @Test
    fun a_project_scope_searches_session_titles_in_that_project() =
        runTest(dispatcher) {
            settings.sessionHits = listOf(SearchHit("s2", "Login", "Login"))
            val vm = model(projectId = "p1", initialQuery = "login")
            advanceUntilIdle()
            assertEquals(SearchScope.THIS_PROJECT, vm.state.value.scope)
            assertEquals(listOf("searchSessions:p1:login"), searchCalls())
        }

    @Test
    fun without_a_project_the_scope_is_all_projects_and_cannot_change() =
        runTest(dispatcher) {
            val vm = model(initialQuery = "login")
            advanceUntilIdle()
            vm.setScope(SearchScope.THIS_PROJECT)
            assertEquals(SearchScope.ALL_PROJECTS, vm.state.value.scope)
        }

    @Test
    fun changing_the_scope_runs_the_query_again_at_once() =
        runTest(dispatcher) {
            val vm = model(projectId = "p1", initialQuery = "login")
            advanceUntilIdle()
            vm.setScope(SearchScope.ALL_PROJECTS)
            advanceUntilIdle()
            assertEquals("searchTranscripts:login:50", searchCalls().last())
        }

    @Test
    fun a_failed_search_keeps_the_failure_and_no_hits() =
        runTest(dispatcher) {
            val vm = model()
            settings.failNext = SessionSettingsException(SessionFailure.UNAVAILABLE)
            vm.onQueryChange("login")
            advanceUntilIdle()
            val state = vm.state.value
            assertEquals(SessionFailure.UNAVAILABLE, state.failure)
            assertTrue(state.hits.isEmpty())
        }
}
