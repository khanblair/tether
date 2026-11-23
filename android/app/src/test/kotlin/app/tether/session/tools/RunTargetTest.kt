package app.tether.session.tools

import app.tether.session.agent.ToolStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RunTargetTest {
    @Test
    fun a_linux_label_at_the_start_is_read() {
        assertEquals(RunTarget(RunExecutor.Linux, "ls"), parseRunTarget("[linux] ls"))
    }

    @Test
    fun a_ci_label_at_the_start_is_read() {
        assertEquals(RunTarget(RunExecutor.Ci, "npm test"), parseRunTarget("[ci] npm test"))
    }

    @Test
    fun a_sandbox_label_at_the_start_is_read() {
        assertEquals(
            RunTarget(RunExecutor.Sandbox, "pytest -q"),
            parseRunTarget("[sandbox] pytest -q"),
        )
    }

    @Test
    fun a_target_with_no_label_has_no_executor() {
        assertEquals(RunTarget(null, "pytest -q"), parseRunTarget("pytest -q"))
    }

    @Test
    fun a_label_in_the_middle_is_not_a_label() {
        assertEquals(RunTarget(null, "echo [linux] ls"), parseRunTarget("echo [linux] ls"))
    }

    @Test
    fun a_bare_label_with_no_command_is_not_a_label() {
        assertEquals(RunTarget(null, "[linux]"), parseRunTarget("[linux]"))
    }

    @Test
    fun empty_output_has_no_lines_and_nothing_hidden() {
        assertEquals(TailLines(emptyList(), 0), tailLines(""))
    }

    @Test
    fun a_trailing_newline_does_not_add_an_empty_line() {
        assertEquals(TailLines(listOf("a", "b"), 0), tailLines("a\nb\n", keep = 12))
    }

    @Test
    fun the_tail_keeps_the_newest_lines_and_counts_the_rest() {
        val output = (1..30).joinToString("\n") { "line $it" }
        val tail = tailLines(output)
        assertEquals(RUN_TAIL_LINES, tail.lines.size)
        assertEquals("line 19", tail.lines.first())
        assertEquals("line 30", tail.lines.last())
        assertEquals(18, tail.hidden)
    }

    @Test
    fun the_tail_of_twelve_lines_or_fewer_hides_nothing() {
        val output = (1..12).joinToString("\n") { "line $it" }
        assertEquals(0, tailLines(output).hidden)
    }

    @Test
    fun run_elsewhere_is_offered_only_for_a_failed_or_timed_out_sandbox_run() {
        assertTrue(canRunElsewhere(ToolStatus.Failed, RunExecutor.Sandbox))
        assertTrue(canRunElsewhere(ToolStatus.TimedOut, RunExecutor.Sandbox))
        assertFalse(canRunElsewhere(ToolStatus.Succeeded, RunExecutor.Sandbox))
        assertFalse(canRunElsewhere(ToolStatus.Failed, RunExecutor.Linux))
        assertFalse(canRunElsewhere(ToolStatus.Failed, RunExecutor.Ci))
        assertFalse(canRunElsewhere(ToolStatus.Failed, null))
    }
}
