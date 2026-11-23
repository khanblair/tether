package app.tether.session.approval

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AlwaysAllowRuleTest {
    private data class Case(
        val name: String,
        val tool: String,
        val target: String,
        val expected: AlwaysAllowRule?,
    )

    private val cases =
        listOf(
            Case("simple command", "run", "ls -la", command("ls")),
            Case("command with a pipe", "run", "cat a.txt | grep b", null),
            Case("command with and", "run", "make && make test", null),
            Case("command with a variable", "run", "echo \$HOME", null),
            Case("command with a redirect", "run", "ls > out.txt", null),
            Case("command with a backslash", "run", "ls \\-la", null),
            Case("label stripped", "run", "[linux] ls -la", command("ls")),
            Case("edit of a relative path", "write_file", "src/app.rs", edit("src/app.rs")),
            Case("edit of an absolute path", "write_file", "/etc/hosts", null),
            Case("edit with a parent segment", "edit_file", "../secret.txt", null),
            Case("move", "move_file", "a.txt -> b.txt", null),
            Case("push", "git_push", "push feature/work to origin", null),
            Case("pull request", "pr_create", "open a pull request", null),
            Case("network", "web_fetch", "https://example.test/docs", null),
            Case("mcp tool", "mcp__docs__read", "docs: read {}", null),
            Case("empty command", "run", "", null),
        )

    @Test
    fun every_case_gets_the_expected_rule() {
        cases.forEach { case ->
            assertEquals(case.name, case.expected, alwaysAllowRule(case.tool, case.target))
        }
    }

    @Test
    fun a_label_with_no_command_gets_no_rule() {
        assertNull(alwaysAllowRule("run", "[linux]"))
        assertNull(alwaysAllowRule("run", "[linux] "))
    }

    @Test
    fun delete_gets_a_rule_for_its_exact_path() {
        assertEquals(edit("notes/todo.md"), alwaysAllowRule("delete_file", "notes/todo.md"))
    }

    @Test
    fun multi_edit_gets_a_rule_and_a_dot_segment_does_not() {
        assertEquals(edit("lib/a.kt"), alwaysAllowRule("multi_edit", "  lib/a.kt  "))
        assertNull(alwaysAllowRule("edit_file", "./lib/a.kt"))
    }

    @Test
    fun a_dot_inside_a_file_name_is_still_a_relative_path() {
        assertEquals(edit("lib/a..b.kt"), alwaysAllowRule("write_file", "lib/a..b.kt"))
    }

    @Test
    fun an_empty_path_gets_no_rule() {
        assertNull(alwaysAllowRule("write_file", ""))
        assertNull(alwaysAllowRule("write_file", "   "))
    }

    private companion object {
        fun command(program: String) = AlwaysAllowRule(AlwaysAllowKind.Command, program)

        fun edit(path: String) = AlwaysAllowRule(AlwaysAllowKind.Edit, path)
    }
}
