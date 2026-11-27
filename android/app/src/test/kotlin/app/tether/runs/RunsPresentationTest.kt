package app.tether.runs

import app.tether.design.components.SessionState
import app.tether.session.tools.RunExecutor
import org.junit.Assert.assertEquals
import org.junit.Test

class RunsPresentationTest {
    @Test
    fun outcomes_map_to_the_shared_status_marks() {
        assertEquals(SessionState.Done, runState(RunOutcome.Succeeded, background = false))
        assertEquals(SessionState.Failed, runState(RunOutcome.TimedOut, background = false))
        assertEquals(SessionState.Failed, runState(RunOutcome.OutOfMemory, background = false))
        assertEquals(SessionState.Idle, runState(RunOutcome.Cancelled, background = false))
        assertEquals(SessionState.Idle, runState(RunOutcome.Denied, background = false))
        assertEquals(SessionState.Running, runState(RunOutcome.Unknown, background = true))
        assertEquals(SessionState.Idle, runState(RunOutcome.Unknown, background = false))
    }

    @Test
    fun metadata_chooses_background_exit_duration_or_outcome_text() {
        assertEquals(RunMetaKind.Background, metaKind(run(background = true)))
        assertEquals(RunMetaKind.ExitAndDuration, metaKind(run()))
        assertEquals(
            RunMetaKind.DurationOnly,
            metaKind(run(exitCode = null)),
        )
        assertEquals(
            RunMetaKind.Outcome,
            metaKind(run(exitCode = null, durationMs = null)),
        )
    }

    @Test
    fun merge_orders_rows_and_drops_local_ci_rows() {
        val localCi = run(callId = "local-ci", startedAtMs = 9_000L, executor = RunExecutor.Ci)
        val local = run(callId = "local", startedAtMs = 1_000L)
        val ci =
            RunRow.Ci(
                id = 4L,
                workflow = "check.yml",
                branch = "main",
                state = app.tether.settings.executors.CiRunState.Succeeded,
                durationSeconds = 1L,
                startedAtMs = 2_000L,
                htmlUrl = "https://example.test/4",
            )

        assertEquals(
            listOf("ci-4", "local"),
            mergeRuns(listOf(RunRow.Local(localCi), RunRow.Local(local)), listOf(ci))
                .map { row ->
                    when (row) {
                        is RunRow.Local -> row.run.callId
                        is RunRow.Ci -> "ci-${row.id}"
                    }
                },
        )
    }

    private fun run(
        callId: String = "run",
        startedAtMs: Long = 2_000L,
        executor: RunExecutor = RunExecutor.Sandbox,
        exitCode: Int? = 0,
        durationMs: Long? = 1_000L,
        background: Boolean = false,
    ): LocalRun =
        LocalRun(
            callId = callId,
            sessionId = "session",
            sessionTitle = "Session",
            command = "echo ok",
            executor = executor,
            outcome = if (background) RunOutcome.Background else RunOutcome.Succeeded,
            exitCode = exitCode,
            durationMs = durationMs,
            startedAtMs = startedAtMs,
            background = background,
        )
}
