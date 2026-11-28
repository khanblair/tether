package app.tether.session.approval

import org.junit.Assert.assertEquals
import org.junit.Test

class ApprovalKindTest {
    @Test
    fun each_tool_maps_to_its_kind() {
        assertEquals(ApprovalKind.Command, approvalKind("run", "ls"))
        assertEquals(
            ApprovalKind.Server,
            approvalKind("run", "[linux] npm run dev (keeps running until stopped)"),
        )
        assertEquals(ApprovalKind.ProcessStop, approvalKind("process_stop", "server-1"))
        assertEquals(ApprovalKind.Edit, approvalKind("write_file", "a.kt"))
        assertEquals(ApprovalKind.Edit, approvalKind("edit_file", "a.kt"))
        assertEquals(ApprovalKind.Edit, approvalKind("multi_edit", "a.kt"))
        assertEquals(ApprovalKind.Delete, approvalKind("delete_file", "a.kt"))
        assertEquals(ApprovalKind.Move, approvalKind("move_file", "a -> b"))
        assertEquals(ApprovalKind.Network, approvalKind("web_fetch", "https://example.test"))
        assertEquals(ApprovalKind.Push, approvalKind("git_push", "push main to origin"))
        assertEquals(ApprovalKind.PullRequest, approvalKind("pr_create", "open a pull request"))
        assertEquals(
            ApprovalKind.GithubRepoCreate,
            approvalKind("github_repo_create", "create GitHub repository blair/app (private)"),
        )
        assertEquals(
            ApprovalKind.GithubApi,
            approvalKind("github_api", "GET https://api.github.com/user/repos"),
        )
        assertEquals(ApprovalKind.Other, approvalKind("mcp__docs__read", "docs: read {}"))
    }

    @Test
    fun a_forced_push_is_a_force_push_and_a_delete_is_a_plain_push() {
        assertEquals(
            ApprovalKind.ForcePush,
            approvalKind("git_push", "FORCE push feature/work to origin (rewrites history)"),
        )
        assertEquals(
            ApprovalKind.Push,
            approvalKind("git_push", "delete branch feature/work on origin"),
        )
    }

    @Test
    fun a_short_text_is_returned_unchanged() {
        assertEquals("ls -la", middleEllipsize("ls -la", 30))
        assertEquals("abcde", middleEllipsize("abcde", 5))
    }

    @Test
    fun a_long_path_keeps_both_ends_with_one_mark_in_the_middle() {
        val cut = middleEllipsize("src/features/approvals/ApprovalViewModel.kt", 20)
        assertEquals(20, cut.length)
        assertEquals("src/featur…wModel.kt", cut)
        assertEquals(1, cut.count { it == '…' })
    }

    @Test(expected = IllegalArgumentException::class)
    fun a_limit_below_one_is_refused() {
        middleEllipsize("abc", 0)
    }

    @Test
    fun a_background_server_target_is_shortened_in_the_approval_bar() {
        assertEquals(
            "npm run dev",
            approvalTarget("run", "[linux] npm run dev (keeps running until stopped)"),
        )
    }
}
