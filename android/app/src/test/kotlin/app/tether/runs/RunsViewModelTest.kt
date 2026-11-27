package app.tether.runs

import app.tether.core.generated.ExecutorView
import app.tether.settings.executors.CiReadiness
import app.tether.settings.executors.CiRunSummary
import app.tether.settings.executors.CiRunsContent
import app.tether.settings.executors.FakeExecutorsGateway
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class RunsViewModelTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun loading_then_rows_asks_for_one_hundred_local_and_ci_rows() {
        val gateway = FakeRunsGateway(mapOf("p" to listOf(local("local", 2_000L))))
        val executors = FakeExecutorsGateway(runs = listOf(ci(1L, 1_000L)))
        val viewModel = RunsViewModel(gateway, executors, "p", dispatcher)

        assertEquals(RunsContent.Loading, viewModel.state.value.content)
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(100, gateway.limits.single())
        assertEquals(listOf(100), executors.runLimits)
        assertEquals(2, rows(viewModel).size)
    }

    @Test
    fun all_rows_are_newest_first() {
        val gateway =
            FakeRunsGateway(
                mapOf(
                    "p" to
                        listOf(
                            local("old", 1_000L),
                            local("new", 3_000L),
                        ),
                ),
            )
        val viewModel = loaded(gateway, FakeExecutorsGateway())

        assertEquals(listOf("new", "old"), rows(viewModel).map { (it as RunRow.Local).run.callId })
    }

    @Test
    fun filters_keep_only_the_selected_executor_and_ci_rows() {
        val gateway =
            FakeRunsGateway(
                mapOf(
                    "p" to
                        listOf(
                            local("sandbox", 3_000L, ExecutorView.SANDBOX),
                            local("linux", 2_000L, ExecutorView.LINUX),
                            local("ci-local", 1_000L, ExecutorView.CI),
                        ),
                ),
            )
        val viewModel = loaded(gateway, FakeExecutorsGateway(runs = listOf(ci(9L, 4_000L))))

        viewModel.setFilter(RunsFilter.Sandbox)
        assertEquals(listOf("sandbox"), localCallIds(viewModel))
        viewModel.setFilter(RunsFilter.Linux)
        assertEquals(listOf("linux"), localCallIds(viewModel))
        viewModel.setFilter(RunsFilter.Ci)
        assertEquals(listOf("ci-9"), rows(viewModel).map { (it as RunRow.Ci).key() })
        viewModel.setFilter(RunsFilter.All)
        assertEquals(listOf("ci-9", "sandbox", "linux"), rows(viewModel).map { it.key() })
    }

    @Test
    fun local_ci_executor_rows_are_not_duplicated_in_all() {
        val gateway =
            FakeRunsGateway(mapOf("p" to listOf(local("ci-local", 2_000L, ExecutorView.CI))))
        val viewModel = loaded(gateway, FakeExecutorsGateway(runs = listOf(ci(7L, 2_500L))))

        assertEquals(listOf("ci-7"), rows(viewModel).map { it.key() })
    }

    @Test
    fun ci_not_ready_keeps_local_rows_in_all_and_explains_the_ci_filter() {
        val gateway = FakeRunsGateway(mapOf("p" to listOf(local("local", 2_000L))))
        val viewModel =
            loaded(
                gateway,
                FakeExecutorsGateway(readiness = CiReadiness.NotSignedIn),
            )

        assertEquals(listOf("local"), localCallIds(viewModel))
        viewModel.setFilter(RunsFilter.Ci)
        assertEquals(RunsContent.Empty, viewModel.state.value.content)
        assertEquals(
            CiRunsContent.NotReady(CiReadiness.NotSignedIn),
            viewModel.state.value.ciContent,
        )
    }

    @Test
    fun no_project_does_not_call_either_gateway() {
        val gateway = FakeRunsGateway()
        val executors = FakeExecutorsGateway()
        val viewModel = RunsViewModel(gateway, executors, null, dispatcher)

        assertEquals(RunsContent.NoProject, viewModel.state.value.content)
        assertTrue(gateway.calls.isEmpty())
        assertTrue(executors.calls.isEmpty())
    }

    @Test
    fun empty_local_and_ci_history_shows_empty() {
        val viewModel = loaded(FakeRunsGateway(), FakeExecutorsGateway())

        assertEquals(RunsContent.Empty, viewModel.state.value.content)
    }

    @Test
    fun local_failure_can_be_retried() {
        val gateway = FakeRunsGateway(failure = RunsFailure.Unavailable)
        val viewModel = RunsViewModel(gateway, FakeExecutorsGateway(), "p", dispatcher)
        dispatcher.scheduler.advanceUntilIdle()
        assertEquals(RunsContent.Failed(RunsFailure.Unavailable), viewModel.state.value.content)

        gateway.failure = null
        gateway.runsByProject = mapOf("p" to listOf(local("retried", 1_000L)))
        viewModel.refresh()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf("retried"), localCallIds(viewModel))
    }

    @Test
    fun switching_project_reloads_the_selected_project() {
        val gateway =
            FakeRunsGateway(
                mapOf(
                    "a" to listOf(local("a-run", 1_000L)),
                    "b" to listOf(local("b-run", 2_000L)),
                ),
            )
        val viewModel = loaded(gateway, FakeExecutorsGateway(), projectId = "a")

        viewModel.selectProject("b")
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals("b", viewModel.state.value.projectId)
        assertEquals(listOf("b-run"), localCallIds(viewModel))
    }

    @Test
    fun the_filter_survives_a_reload() {
        val gateway =
            FakeRunsGateway(
                mapOf(
                    "p" to
                        listOf(
                            local("sandbox", 2_000L, ExecutorView.SANDBOX),
                            local("linux", 1_000L, ExecutorView.LINUX),
                        ),
                ),
            )
        val viewModel = loaded(gateway, FakeExecutorsGateway())
        viewModel.setFilter(RunsFilter.Linux)
        viewModel.refresh()
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(RunsFilter.Linux, viewModel.state.value.filter)
        assertEquals(listOf("linux"), localCallIds(viewModel))
    }

    private fun loaded(
        gateway: FakeRunsGateway,
        executors: FakeExecutorsGateway,
        projectId: String = "p",
    ): RunsViewModel =
        RunsViewModel(gateway, executors, projectId, dispatcher).also {
            dispatcher.scheduler.advanceUntilIdle()
        }

    private fun rows(viewModel: RunsViewModel): List<RunRow> =
        (viewModel.state.value.content as RunsContent.Runs).rows

    private fun localCallIds(viewModel: RunsViewModel): List<String> =
        rows(viewModel).filterIsInstance<RunRow.Local>().map { it.run.callId }

    private fun local(
        callId: String,
        startedAtMs: Long,
        executor: ExecutorView? = ExecutorView.SANDBOX,
    ): LocalRun =
        LocalRun(
            callId = callId,
            sessionId = "session-$callId",
            sessionTitle = "Session $callId",
            command = "echo $callId",
            executor = executor.toRunExecutor(),
            outcome = RunOutcome.Succeeded,
            exitCode = 0,
            durationMs = 1_000L,
            startedAtMs = startedAtMs,
            background = false,
        )

    private fun ci(
        id: Long,
        startedAtMs: Long,
    ): CiRunSummary =
        CiRunSummary(
            id = id,
            title = "workflow.yml",
            branch = "main",
            status = "completed",
            conclusion = "success",
            startedAt =
                java.time.Instant
                    .ofEpochMilli(startedAtMs)
                    .toString(),
            durationSeconds = 6L,
            htmlUrl = "https://example.test/$id",
        )

    private fun ExecutorView?.toRunExecutor(): app.tether.session.tools.RunExecutor? =
        when (this) {
            ExecutorView.SANDBOX -> app.tether.session.tools.RunExecutor.Sandbox
            ExecutorView.LINUX -> app.tether.session.tools.RunExecutor.Linux
            ExecutorView.CI -> app.tether.session.tools.RunExecutor.Ci
            null -> null
        }

    private fun RunRow.key(): String =
        when (this) {
            is RunRow.Local -> run.callId
            is RunRow.Ci -> "ci-$id"
        }
}
