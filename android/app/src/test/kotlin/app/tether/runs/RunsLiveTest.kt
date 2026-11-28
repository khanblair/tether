package app.tether.runs

import app.tether.design.components.SessionState
import app.tether.settings.executors.FakeExecutorsGateway
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

/** Which background runs still have a process, and what the row shows for each. */
@OptIn(ExperimentalCoroutinesApi::class)
class RunsLiveTest {
    private val dispatcher = StandardTestDispatcher()

    @Before
    fun setUp() = Dispatchers.setMain(dispatcher)

    @After
    fun tearDown() = Dispatchers.resetMain()

    @Test
    fun a_background_run_matches_the_process_of_its_session_and_command() {
        val run = background("call-1", "s1", "npm run dev", 1_000L)

        val matched = matchLive(listOf(run), listOf(process("s1", "p1", "npm run dev", 1_200L)))

        assertEquals(LiveProcess("s1", "p1"), matched["call-1"])
    }

    @Test
    fun a_process_of_another_session_or_command_does_not_match() {
        val run = background("call-1", "s1", "npm run dev", 1_000L)

        val matched =
            matchLive(
                listOf(run),
                listOf(
                    process("s2", "p1", "npm run dev", 1_000L),
                    process("s1", "p2", "npm start", 1_000L),
                ),
            )

        assertEquals(emptyMap<String, LiveProcess>(), matched)
    }

    @Test
    fun two_runs_of_one_command_take_one_process_each_by_nearest_start() {
        val old = background("old", "s1", "npm run dev", 1_000L)
        val new = background("new", "s1", "npm run dev", 9_000L)
        val processes =
            listOf(
                process("s1", "p1", "npm run dev", 1_100L),
                process("s1", "p2", "npm run dev", 9_100L),
            )

        val matched = matchLive(listOf(old, new), processes)

        assertEquals(LiveProcess("s1", "p2"), matched["new"])
        assertEquals(LiveProcess("s1", "p1"), matched["old"])
    }

    @Test
    fun one_process_serves_only_the_newest_of_two_runs() {
        val old = background("old", "s1", "npm run dev", 1_000L)
        val new = background("new", "s1", "npm run dev", 9_000L)

        val only = process("s1", "p1", "npm run dev", 9_100L)

        val matched = matchLive(listOf(old, new), listOf(only))

        assertEquals(setOf("new"), matched.keys)
    }

    @Test
    fun a_command_cut_to_500_characters_in_the_history_still_matches() {
        val long = "x".repeat(600)
        val run = background("call-1", "s1", long.take(500), 1_000L)

        val matched = matchLive(listOf(run), listOf(process("s1", "p1", long, 1_000L)))

        assertEquals(LiveProcess("s1", "p1"), matched["call-1"])
    }

    @Test
    fun a_finished_run_never_matches_a_process() {
        val run =
            background("call-1", "s1", "npm test", 1_000L)
                .copy(background = false, outcome = RunOutcome.Succeeded)

        val matched = matchLive(listOf(run), listOf(process("s1", "p1", "npm test", 1_000L)))

        assertEquals(emptyMap<String, LiveProcess>(), matched)
    }

    @Test
    fun the_row_status_follows_the_matched_process() {
        val run = background("call-1", "s1", "npm run dev", 1_000L)

        val live = RunRow.Local(run, LiveProcess("s1", "p1"))
        val gone = RunRow.Local(run)
        val done = RunRow.Local(run.copy(background = false, outcome = RunOutcome.Succeeded))

        assertEquals(LocalRunStatus.Live, live.status())
        assertEquals(SessionState.Running, live.markState())
        assertEquals(LocalRunStatus.Stopped, gone.status())
        assertEquals(SessionState.Idle, gone.markState())
        assertEquals(LocalRunStatus.Finished, done.status())
        assertEquals(SessionState.Done, done.markState())
    }

    @Test
    fun the_view_model_marks_rows_live_or_stopped_from_the_open_processes() {
        val processes =
            FakeProcesses(listOf(process("s1", "p1", "npm run dev", 1_100L)))
        val viewModel =
            loaded(
                listOf(
                    background("live", "s1", "npm run dev", 1_000L),
                    background("dead", "s1", "npm run dev", 500L),
                ),
                processes,
            )

        val rows = viewModel.localRows().associateBy { it.run.callId }

        assertEquals(LiveProcess("s1", "p1"), rows.getValue("live").live)
        assertNull(rows.getValue("dead").live)
    }

    @Test
    fun stopping_a_live_row_stops_its_process_and_reads_the_history_again() {
        val processes = FakeProcesses(listOf(process("s1", "p1", "npm run dev", 1_100L)))
        val viewModel = loaded(listOf(background("live", "s1", "npm run dev", 1_000L)), processes)

        viewModel.stop(viewModel.localRows().single())
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(listOf("s1/p1"), processes.stopped)
        assertNull(viewModel.localRows().single().live)
    }

    @Test
    fun stopping_a_row_without_a_process_does_nothing() {
        val processes = FakeProcesses(emptyList())
        val viewModel = loaded(listOf(background("dead", "s1", "npm run dev", 1_000L)), processes)

        viewModel.stop(viewModel.localRows().single())
        dispatcher.scheduler.advanceUntilIdle()

        assertEquals(emptyList<String>(), processes.stopped)
    }

    private fun loaded(
        runs: List<LocalRun>,
        processes: FakeProcesses,
    ): RunsViewModel =
        RunsViewModel(
            FakeRunsGateway(mapOf("p" to runs)),
            FakeExecutorsGateway(),
            "p",
            dispatcher,
            processes,
        ).also { dispatcher.scheduler.advanceUntilIdle() }

    private fun RunsViewModel.localRows(): List<RunRow.Local> =
        (state.value.content as RunsContent.Runs).rows.filterIsInstance<RunRow.Local>()

    private fun background(
        callId: String,
        sessionId: String,
        command: String,
        startedAtMs: Long,
    ): LocalRun =
        LocalRun(
            callId = callId,
            sessionId = sessionId,
            sessionTitle = "Session $sessionId",
            command = command,
            executor = null,
            outcome = RunOutcome.Background,
            exitCode = null,
            durationMs = null,
            startedAtMs = startedAtMs,
            background = true,
        )

    private fun process(
        sessionId: String,
        processId: String,
        command: String,
        startedAtMs: Long,
    ) = OpenProcess(sessionId, processId, command, startedAtMs)
}

/** Open processes that a stop removes, like the real registry after a process ends. */
private class FakeProcesses(
    initial: List<OpenProcess>,
) : RunsProcesses {
    private var open = initial
    val stopped = mutableListOf<String>()

    override fun open(): List<OpenProcess> = open

    override fun stop(
        sessionId: String,
        processId: String,
    ) {
        stopped += "$sessionId/$processId"
        open = open.filterNot { it.sessionId == sessionId && it.processId == processId }
    }
}
