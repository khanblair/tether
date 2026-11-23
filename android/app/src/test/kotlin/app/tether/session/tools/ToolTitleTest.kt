package app.tether.session.tools

import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus
import org.junit.Assert.assertEquals
import org.junit.Test

class ToolTitleTest {
    @Test
    fun core_tool_names_map_to_their_title_kind() {
        val table =
            mapOf(
                "read_file" to ToolTitleKind.ReadFile,
                "write_file" to ToolTitleKind.WriteFile,
                "edit_file" to ToolTitleKind.EditFile,
                "delete_file" to ToolTitleKind.DeleteFile,
                "list_dir" to ToolTitleKind.ListDir,
                "glob" to ToolTitleKind.Search,
                "grep" to ToolTitleKind.Search,
                "run" to ToolTitleKind.Run,
                "ci_run" to ToolTitleKind.CiRun,
                "ci_logs" to ToolTitleKind.CiLogs,
                "git_status" to ToolTitleKind.GitStatus,
                "git_diff" to ToolTitleKind.GitDiff,
                "git_log" to ToolTitleKind.GitLog,
                "git_commit" to ToolTitleKind.GitCommit,
                "git_push" to ToolTitleKind.GitPush,
                "git_branch" to ToolTitleKind.GitBranch,
                "pr_create" to ToolTitleKind.PrCreate,
                "pr_status" to ToolTitleKind.PrStatus,
            )
        table.forEach { (tool, kind) -> assertEquals(tool, kind, toolTitleKind(tool)) }
    }

    @Test
    fun an_unknown_tool_name_gives_the_unknown_kind() {
        assertEquals(ToolTitleKind.Unknown, toolTitleKind("mystery_tool"))
        assertEquals(ToolTitleKind.Unknown, toolTitleKind(""))
    }

    @Test
    fun running_is_present_done_is_past_and_waiting_is_imperative() {
        assertEquals(ToolTense.Present, toolTense(ToolStatus.Running))
        assertEquals(ToolTense.Past, toolTense(ToolStatus.Succeeded))
        assertEquals(ToolTense.Past, toolTense(ToolStatus.Failed))
        assertEquals(ToolTense.Past, toolTense(ToolStatus.TimedOut))
        assertEquals(ToolTense.Past, toolTense(ToolStatus.Denied))
        assertEquals(ToolTense.Past, toolTense(ToolStatus.Blocked))
        assertEquals(ToolTense.Past, toolTense(ToolStatus.Cancelled))
        assertEquals(ToolTense.Imperative, toolTense(ToolStatus.AwaitingApproval))
        assertEquals(ToolTense.Imperative, toolTense(ToolStatus.Proposed))
        assertEquals(ToolTense.Imperative, toolTense(ToolStatus.Allowed))
    }

    @Test
    fun a_read_keeps_its_target_and_takes_the_tense_of_its_status() {
        val call = ToolCallUi("1", "read_file", "src/app.py", ToolStatus.Running, "", false)
        assertEquals(
            ToolTitle(ToolTitleKind.ReadFile, ToolTense.Present, "src/app.py", "read_file"),
            toolTitle(call),
        )
    }

    @Test
    fun a_run_title_drops_the_executor_label() {
        val call = ToolCallUi("2", "run", "[linux] pytest -q", ToolStatus.Succeeded, "", false)
        assertEquals("pytest -q", toolTitle(call).target)
        assertEquals(ToolTense.Past, toolTitle(call).tense)
    }

    @Test
    fun an_unknown_tool_keeps_its_name_for_the_fallback_title() {
        val call = ToolCallUi("3", "mystery_tool", "arg", ToolStatus.Succeeded, "", false)
        val title = toolTitle(call)
        assertEquals(ToolTitleKind.Unknown, title.kind)
        assertEquals("mystery_tool", title.tool)
    }
}
