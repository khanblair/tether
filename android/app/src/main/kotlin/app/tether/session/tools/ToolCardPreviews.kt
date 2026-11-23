package app.tether.session.tools

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus

private const val SAMPLE_OUTPUT =
    "tests/test_login.py ..F.                  [ 80%]\n" +
        "================= FAILURES =================\n" +
        "_____ test_token_expires_at_boundary _____\n" +
        "assert is_expired(token, now=100)\n" +
        "E   AssertionError: expected True\n" +
        "1 failed, 3 passed in 4.8s"

private fun call(
    id: String,
    tool: String,
    target: String,
    status: ToolStatus,
    output: String = "",
    truncated: Boolean = false,
) = ToolCallUi(id, tool, target, status, output, truncated)

@Preview(name = "Tool statuses, light", showBackground = true)
@Composable
internal fun ToolStatusesLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { ToolStatusesSample() }
}

@Preview(name = "Tool statuses, dark", showBackground = true)
@Composable
internal fun ToolStatusesDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { ToolStatusesSample() }
}

@Composable
private fun ToolStatusesSample() {
    Column(
        modifier = Modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        ToolStatus.entries.forEach { status ->
            ToolCallCard(
                call = call("status-${status.name}", "edit_file", "src/auth/session.py", status),
                expanded = false,
                onToggle = {},
                animateRunning = status == ToolStatus.Running,
            )
        }
    }
}

@Preview(name = "Tool output, light", showBackground = true)
@Composable
internal fun ToolOutputLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { ToolOutputSample() }
}

@Preview(name = "Tool output, dark", showBackground = true)
@Composable
internal fun ToolOutputDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { ToolOutputSample() }
}

@Composable
private fun ToolOutputSample() {
    Column(
        modifier = Modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        ToolCallCard(
            call =
                call(
                    "out-1",
                    "grep",
                    "is_expired",
                    ToolStatus.Succeeded,
                    "src/auth/session.py:43",
                    false,
                ),
            expanded = true,
            onToggle = {},
        )
        ToolCallCard(
            call =
                call(
                    "out-2",
                    "write_file",
                    "tests/test_login.py",
                    ToolStatus.Succeeded,
                    SAMPLE_OUTPUT,
                    true,
                ),
            expanded = true,
            onToggle = {},
        )
        ToolCallCard(
            call =
                call(
                    "out-3",
                    "read_file",
                    "src/auth/session.py",
                    ToolStatus.Failed,
                    "No such file",
                    false,
                ),
            expanded = false,
            onToggle = {},
        )
    }
}

@Preview(name = "Read group, light", showBackground = true)
@Composable
internal fun ToolGroupLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { ToolGroupSample(expanded = false) }
}

@Preview(name = "Read group expanded, dark", showBackground = true)
@Composable
internal fun ToolGroupDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { ToolGroupSample(expanded = true) }
}

@Composable
private fun ToolGroupSample(expanded: Boolean) {
    Column(modifier = Modifier.padding(TetherSpace.space4)) {
        ToolGroupCard(
            calls =
                listOf(
                    call("g-1", "read_file", "src/auth/session.py", ToolStatus.Succeeded),
                    call("g-2", "read_file", "src/auth/tokens.py", ToolStatus.Succeeded),
                    call("g-3", "glob", "**/*login*", ToolStatus.Succeeded),
                    call("g-4", "git_status", "", ToolStatus.Succeeded),
                ),
            groupId = "g-1",
            expanded = expanded,
            onToggleGroup = {},
            isCallExpanded = { false },
            onToggleCall = {},
        )
    }
}

@Preview(name = "Run cards, light", showBackground = true)
@Composable
internal fun RunCardsLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { RunCardsSample() }
}

@Preview(name = "Run cards, dark", showBackground = true)
@Composable
internal fun RunCardsDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { RunCardsSample() }
}

@Composable
private fun RunCardsSample() {
    Column(
        modifier = Modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        RunCard(
            call = call("run-1", "run", "[linux] pytest -q", ToolStatus.Running, SAMPLE_OUTPUT),
            expanded = false,
            onToggle = {},
            animateRunning = true,
        )
        RunCard(
            call =
                call(
                    "run-2",
                    "run",
                    "[sandbox] pip install -e .",
                    ToolStatus.Failed,
                    SAMPLE_OUTPUT,
                    true,
                ),
            expanded = true,
            onToggle = {},
            onRunElsewhere = {},
        )
        RunCard(
            call = call("run-3", "run", "[ci] npm test", ToolStatus.TimedOut, "still waiting"),
            expanded = false,
            onToggle = {},
        )
        RunCard(
            call = call("run-4", "run", "ls -la", ToolStatus.Succeeded, "total 8\n. ..", false),
            expanded = false,
            onToggle = {},
        )
    }
}
