package app.tether.settings.executors

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CiRunsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    private fun viewModel(
        gateway: FakeExecutorsGateway,
        projectId: String? = "tether",
    ): CiRunsViewModel =
        CiRunsViewModel(gateway, projectId, dispatcher).also {
            dispatcher.scheduler.advanceUntilIdle()
        }

    @Test
    fun the_list_asks_the_core_for_the_last_twenty_runs() {
        val gateway = FakeExecutorsGateway(runs = listOf(run(1L)))

        viewModel(gateway)

        assertEquals(listOf(20), gateway.runLimits)
    }

    @Test
    fun each_run_keeps_its_order_and_maps_its_state_and_start_time() {
        val gateway =
            FakeExecutorsGateway(
                runs =
                    listOf(
                        run(
                            2L,
                            status = "completed",
                            conclusion = "success",
                            startedAt = "2026-11-22T10:00:00Z",
                        ),
                        run(1L, status = "in_progress", startedAt = "not a time"),
                    ),
            )

        val content = viewModel(gateway).state.value.content as CiRunsContent.Runs

        assertEquals(listOf(2L, 1L), content.rows.map { it.id })
        assertEquals(CiRunState.Succeeded, content.rows[0].state)
        assertEquals(CiRunState.Running, content.rows[1].state)
        assertEquals(parseStartedAt("2026-11-22T10:00:00Z"), content.rows[0].startedAtMs)
        assertEquals(null, content.rows[1].startedAtMs)
    }

    @Test
    fun a_project_with_no_runs_shows_the_empty_state() {
        val vm = viewModel(FakeExecutorsGateway(runs = emptyList()))

        assertEquals(CiRunsContent.Empty, vm.state.value.content)
    }

    @Test
    fun a_project_whose_workflow_is_missing_shows_the_reason_and_asks_for_no_runs() {
        val gateway =
            FakeExecutorsGateway(readiness = CiReadiness.WorkflowMissing, runs = listOf(run(1L)))

        val vm = viewModel(gateway)

        assertEquals(CiRunsContent.NotReady(CiReadiness.WorkflowMissing), vm.state.value.content)
        assertFalse("ciRuns" in gateway.calls)
    }

    @Test
    fun a_sign_in_missing_from_the_run_list_is_a_reason_not_a_failure() {
        val gateway = FakeExecutorsGateway(runsFailure = ExecutorsFailure.NotSignedIn)

        val vm = viewModel(gateway)

        assertEquals(CiRunsContent.NotReady(CiReadiness.NotSignedIn), vm.state.value.content)
    }

    @Test
    fun a_missing_github_remote_from_the_run_list_is_a_reason() {
        val gateway = FakeExecutorsGateway(runsFailure = ExecutorsFailure.NoRemote)

        val vm = viewModel(gateway)

        assertEquals(CiRunsContent.NotReady(CiReadiness.NoRemote), vm.state.value.content)
    }

    @Test
    fun a_github_error_is_a_failure_without_its_detail() {
        val vm = viewModel(FakeExecutorsGateway(runsFailure = ExecutorsFailure.GithubError))

        assertEquals(CiRunsContent.Failed(ExecutorsFailure.GithubError), vm.state.value.content)
    }

    @Test
    fun refresh_reads_the_runs_again_and_clears_the_refreshing_flag() {
        val gateway = FakeExecutorsGateway(runs = listOf(run(1L)))
        val vm = viewModel(gateway)

        gateway.runs = listOf(run(2L), run(1L))
        vm.refresh()
        assertTrue(vm.state.value.refreshing)
        dispatcher.scheduler.advanceUntilIdle()

        assertFalse(vm.state.value.refreshing)
        assertEquals(
            listOf(2L, 1L),
            (vm.state.value.content as CiRunsContent.Runs).rows.map { it.id },
        )
        assertEquals(listOf(20, 20), gateway.runLimits)
    }

    @Test
    fun without_a_project_the_screen_says_so_and_calls_nothing() {
        val gateway = FakeExecutorsGateway(runs = listOf(run(1L)))

        val vm = viewModel(gateway, projectId = null)

        assertEquals(CiRunsContent.NoProject, vm.state.value.content)
        assertTrue(gateway.calls.isEmpty())
    }

    private fun run(
        id: Long,
        status: String = "completed",
        conclusion: String = "success",
        startedAt: String = "2026-11-22T10:00:00Z",
    ): CiRunSummary =
        CiRunSummary(
            id = id,
            title = "Run $id",
            branch = "main",
            status = status,
            conclusion = conclusion,
            startedAt = startedAt,
            durationSeconds = 64L,
            htmlUrl = "https://example.test/runs/$id",
        )
}
