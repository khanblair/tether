package app.tether.session.approval

import android.content.res.Configuration
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentLoopFailure
import app.tether.session.agent.ApprovalUi

/** A reason long enough for the characters-left counter. */
private const val NEAR_LIMIT = 180
private const val REPEATS = 8
private const val LONG_PATH = "src/features/approvals/nested/ApprovalViewModel.kt"

private val commandApproval = ApprovalUi("t1", "run", "[linux] pip install -e . && pytest -q")
private val editApproval = ApprovalUi("t2", "edit_file", LONG_PATH)
private val networkApproval = ApprovalUi("t3", "web_fetch", "https://example.test/docs/limits")
private val pushApproval = ApprovalUi("t4", "git_push", "push feature/work to origin")
private val forcePushApproval =
    ApprovalUi("t5", "git_push", "FORCE push feature/work to origin (rewrites history)")

/** A bar with the given approval and state. Every callback is a no-op. */
@Composable
internal fun PreviewBar(
    approval: ApprovalUi,
    expanded: Boolean = false,
    position: Int = 1,
    total: Int = 1,
    busy: Boolean = false,
    failure: AgentLoopFailure? = null,
) {
    Column(modifier = Modifier.padding(TetherSpace.space4)) {
        ApprovalBar(
            approval = approval,
            position = position,
            total = total,
            expanded = expanded,
            busy = busy,
            failure = failure,
            onToggleExpanded = {},
            onAllow = {},
            onDeny = {},
            onAlwaysAllow = {},
        )
    }
}

@Preview(name = "Bar: command, light", showBackground = true)
@Composable
internal fun BarCommandLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { PreviewBar(commandApproval) }
}

@Preview(
    name = "Bar: command, dark",
    uiMode = Configuration.UI_MODE_NIGHT_YES,
    showBackground = true,
)
@Composable
internal fun BarCommandDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { PreviewBar(commandApproval, expanded = true) }
}

@Preview(name = "Bar: edit, one of three, light", showBackground = true)
@Composable
internal fun BarEditLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { PreviewBar(editApproval, position = 1, total = 3) }
}

@Preview(
    name = "Bar: edit, expanded, dark",
    uiMode = Configuration.UI_MODE_NIGHT_YES,
    showBackground = true,
)
@Composable
internal fun BarEditDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { PreviewBar(editApproval, expanded = true, total = 3) }
}

@Preview(name = "Bar: network, light", showBackground = true)
@Composable
internal fun BarNetworkLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { PreviewBar(networkApproval) }
}

@Preview(name = "Bar: push, expanded, light", showBackground = true)
@Composable
internal fun BarPushLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { PreviewBar(pushApproval, expanded = true) }
}

@Preview(
    name = "Bar: force push, dark",
    uiMode = Configuration.UI_MODE_NIGHT_YES,
    showBackground = true,
)
@Composable
internal fun BarForcePushDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { PreviewBar(forcePushApproval, expanded = true) }
}

@Preview(name = "Bar: failed and busy, light", showBackground = true)
@Composable
internal fun BarFailureLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        PreviewBar(commandApproval, failure = AgentLoopFailure.Unavailable, busy = true)
    }
}

@Preview(name = "Always allow: command, light", showBackground = true)
@Composable
internal fun AlwaysCommandLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AlwaysAllowMenuContent(
            rule = AlwaysAllowRule(AlwaysAllowKind.Command, "ls"),
            busy = false,
            failure = null,
            onConfirm = {},
            onCancel = {},
        )
    }
}

@Preview(
    name = "Always allow: edit, dark",
    uiMode = Configuration.UI_MODE_NIGHT_YES,
    showBackground = true,
)
@Composable
internal fun AlwaysEditDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AlwaysAllowMenuContent(
            rule = AlwaysAllowRule(AlwaysAllowKind.Edit, "src/app.rs"),
            busy = false,
            failure = AgentLoopFailure.NotFound,
            onConfirm = {},
            onCancel = {},
        )
    }
}

@Preview(name = "Deny: empty, light", showBackground = true)
@Composable
internal fun DenyEmptyLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        DenyReasonContent(
            reason = "",
            busy = false,
            failure = null,
            onReasonChange = {},
            onSubmit = {},
            onCancel = {},
        )
    }
}

@Preview(
    name = "Deny: filled near limit, dark",
    uiMode = Configuration.UI_MODE_NIGHT_YES,
    showBackground = true,
)
@Composable
internal fun DenyFilledDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        DenyReasonContent(
            reason = "Use the sandbox instead of the host. ".repeat(REPEATS).take(NEAR_LIMIT),
            busy = false,
            failure = null,
            onReasonChange = {},
            onSubmit = {},
            onCancel = {},
        )
    }
}
