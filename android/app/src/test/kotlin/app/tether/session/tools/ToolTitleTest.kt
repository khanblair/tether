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
                "process_list" to ToolTitleKind.ProcessList,
                "process_logs" to ToolTitleKind.ProcessLogs,
                "process_stop" to ToolTitleKind.ProcessStop,
                "preview" to ToolTitleKind.Preview,
                "ci_run" to ToolTitleKind.CiRun,
                "ci_logs" to ToolTitleKind.CiLogs,
                "git_status" to ToolTitleKind.GitStatus,
                "git_diff" to ToolTitleKind.GitDiff,
                "git_log" to ToolTitleKind.GitLog,
                "git_commit" to ToolTitleKind.GitCommit,
                "git_push" to ToolTitleKind.GitPush,
                "git_branch" to ToolTitleKind.GitBranch,
                "git_init" to ToolTitleKind.GitInit,
                "github_repo_create" to ToolTitleKind.GithubRepoCreate,
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
    fun a_background_run_is_a_server_title_with_the_command_target() {
        val call =
            ToolCallUi(
                "6",
                "run",
                "[linux] npm run dev (keeps running until stopped)",
                ToolStatus.Succeeded,
                "",
                false,
            )
        val title = toolTitle(call)
        assertEquals(ToolTitleKind.RunServer, title.kind)
        assertEquals("npm run dev", title.target)
    }

    @Test
    fun process_tools_and_preview_keep_human_targets() {
        assertEquals("", toolTitle(call("process_list", "processes")).target)
        assertEquals("abc", toolTitle(call("process_logs", "abc")).target)
        assertEquals("abc", toolTitle(call("process_stop", "abc")).target)
        assertEquals("index.html", toolTitle(call("preview", "index.html")).target)
        assertEquals(
            "the server on port 5173",
            toolTitle(call("preview", "http://127.0.0.1:5173/")).target,
        )
    }

    @Test
    fun a_saved_background_run_rebuilds_the_server_target() {
        assertEquals(
            "[linux] npm run dev (keeps running until stopped)",
            savedToolTarget(
                "run",
                "{\"command\":\"npm run dev\",\"executor\":\"linux\",\"background\":true}",
                "",
            ),
        )
    }

    @Test
    fun an_unknown_tool_keeps_its_name_for_the_fallback_title() {
        val call = ToolCallUi("3", "mystery_tool", "arg", ToolStatus.Succeeded, "", false)
        val title = toolTitle(call)
        assertEquals(ToolTitleKind.Unknown, title.kind)
        assertEquals("mystery_tool", title.tool)
    }

    @Test
    fun git_init_title_keeps_the_branch_in_its_target() {
        val call =
            ToolCallUi(
                "4",
                "git_init",
                "initialize a Git repository on develop",
                ToolStatus.Succeeded,
                "Initialized Git repository on develop",
                false,
            )

        assertEquals(ToolTitleKind.GitInit, toolTitle(call).kind)
        assertEquals("on develop", toolTitle(call).target)
    }

    @Test
    fun a_live_public_repository_create_keeps_owner_visibility_and_web_url() {
        val call =
            ToolCallUi(
                "5",
                "github_repo_create",
                "create GitHub repository blair/app (public)",
                ToolStatus.Succeeded,
                "Created GitHub repository: clone " +
                    "https://github.com/blair/app.git, web https://github.com/blair/app",
                false,
            )

        assertEquals(ToolTitleKind.GithubRepoCreate, toolTitle(call).kind)
        assertEquals(
            "blair/app · public · https://github.com/blair/app",
            toolTitle(call).target,
        )
    }

    private fun call(
        tool: String,
        target: String,
    ) = ToolCallUi("server-$tool", tool, target, ToolStatus.Succeeded, "", false)
}
