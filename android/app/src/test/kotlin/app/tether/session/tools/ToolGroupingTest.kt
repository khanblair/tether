package app.tether.session.tools

import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ToolGroupingTest {
    private fun call(
        id: String,
        tool: String,
        status: ToolStatus = ToolStatus.Succeeded,
    ) = ToolCallUi(id, tool, "target-$id", status, "", false)

    private fun ids(rows: List<CallRow>): List<String> =
        rows.flatMap { row ->
            when (row) {
                is CallRow.Single -> listOf(row.call.id)
                is CallRow.Group -> row.calls.map { it.id }
            }
        }

    @Test
    fun a_single_read_stays_a_single_row() {
        val rows = groupCalls(listOf(call("a", "read_file")))
        assertEquals(listOf<CallRow>(CallRow.Single(call("a", "read_file"))), rows)
    }

    @Test
    fun a_pair_of_reads_becomes_a_group() {
        val rows = groupCalls(listOf(call("a", "read_file"), call("b", "grep")))
        assertEquals(1, rows.size)
        assertEquals(listOf("a", "b"), ids(rows))
        assertTrue(rows.single() is CallRow.Group)
    }

    @Test
    fun a_call_awaiting_approval_breaks_a_group() {
        val rows =
            groupCalls(
                listOf(
                    call("a", "read_file"),
                    call("b", "glob"),
                    call("c", "read_file", ToolStatus.AwaitingApproval),
                    call("d", "read_file"),
                ),
            )
        assertEquals(3, rows.size)
        assertEquals(listOf("a", "b"), ids(listOf(rows[0])))
        assertTrue(rows[0] is CallRow.Group)
        assertEquals(CallRow.Single(call("c", "read_file", ToolStatus.AwaitingApproval)), rows[1])
        assertEquals(CallRow.Single(call("d", "read_file")), rows[2])
    }

    @Test
    fun a_failed_read_breaks_a_group_and_stays_single() {
        val rows =
            groupCalls(
                listOf(
                    call("a", "read_file"),
                    call("b", "read_file", ToolStatus.Failed),
                    call("c", "read_file"),
                ),
            )
        assertEquals(3, rows.size)
        assertTrue(rows.all { it is CallRow.Single })
    }

    @Test
    fun a_running_read_stays_visible_on_its_own() {
        val rows =
            groupCalls(
                listOf(
                    call("a", "read_file"),
                    call("b", "read_file"),
                    call("c", "read_file", ToolStatus.Running),
                ),
            )
        assertEquals(2, rows.size)
        assertTrue(rows[0] is CallRow.Group)
        assertEquals(CallRow.Single(call("c", "read_file", ToolStatus.Running)), rows[1])
    }

    @Test
    fun a_write_between_reads_splits_them_and_keeps_the_order() {
        val calls =
            listOf(
                call("a", "read_file"),
                call("b", "list_dir"),
                call("c", "edit_file"),
                call("d", "git_status"),
                call("e", "pr_status"),
                call("f", "run"),
            )
        val rows = groupCalls(calls)
        assertEquals(listOf("a", "b", "c", "d", "e", "f"), ids(rows))
        assertEquals(4, rows.size)
        assertTrue(rows[0] is CallRow.Group)
        assertEquals(CallRow.Single(call("c", "edit_file")), rows[1])
        assertTrue(rows[2] is CallRow.Group)
        assertEquals(CallRow.Single(call("f", "run")), rows[3])
    }

    @Test
    fun a_timed_out_or_blocked_call_breaks_a_group() {
        val rows =
            groupCalls(
                listOf(
                    call("a", "grep"),
                    call("b", "grep", ToolStatus.TimedOut),
                    call("c", "grep", ToolStatus.Blocked),
                ),
            )
        assertEquals(3, rows.size)
    }

    @Test
    fun the_summary_counts_each_kind_of_read() {
        val summary =
            groupSummary(
                listOf(
                    call("a", "read_file"),
                    call("b", "read_file"),
                    call("c", "read_file"),
                    call("d", "read_file"),
                    call("e", "grep"),
                    call("f", "glob"),
                    call("g", "list_dir"),
                    call("h", "git_log"),
                ),
            )
        assertEquals(GroupSummary(files = 4, folders = 1, searches = 2, checks = 1), summary)
    }

    @Test
    fun the_summary_parts_skip_the_zero_counts() {
        val summary = GroupSummary(files = 4, folders = 0, searches = 2, checks = 0)
        assertEquals(
            listOf(GroupPart.Files to 4, GroupPart.Searches to 2),
            summary.parts(),
        )
    }
}
