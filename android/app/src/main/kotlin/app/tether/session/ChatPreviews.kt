package app.tether.session

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import app.tether.core.generated.MessagePartView
import app.tether.core.generated.MessageView
import app.tether.core.generated.RoleView
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolCardState
import app.tether.session.agent.ToolStatus

private const val NARROW_WIDTH = 320
private const val LARGE_FONT_SCALE = 2f
private const val LONG_TEXT =
    "Can you check why the login test fails on CI but passes on my phone? " +
        "It started after the last change to the session store, and I want the root cause " +
        "before we touch the retry code, so please look at the logs first."

private val sample =
    ToolCardState(
        persisted =
            mapOf(
                "call1" to
                    ToolCallUi("call1", "read", "index.html", ToolStatus.Proposed, "", false),
            ),
        expanded = emptySet(),
        onToggle = {},
    )

private fun message(
    id: String,
    role: RoleView,
    vararg parts: MessagePartView,
) = MessageView(
    id = id,
    sessionId = "preview",
    role = role,
    seq = 1u,
    createdAtMs = 0L,
    rewound = false,
    parts = parts.toList(),
)

@Composable
private fun ChatPreview(
    fontScale: Float = 1f,
    widthDp: Int? = null,
    content: @Composable () -> Unit,
) {
    val density = LocalDensity.current
    val frame = if (widthDp == null) Modifier.fillMaxWidth() else Modifier.width(widthDp.dp)
    TetherTheme(mode = ThemeMode.Light) {
        CompositionLocalProvider(LocalDensity provides Density(density.density, fontScale)) {
            Column(modifier = frame) { content() }
        }
    }
}

@Preview(name = "User message, short", showBackground = true)
@Composable
internal fun UserBubbleShortPreview() {
    ChatPreview {
        TranscriptMessage(message("u1", RoleView.USER, MessagePartView.Text("Fix the test")))
    }
}

@Preview(name = "User message, long", showBackground = true)
@Composable
internal fun UserBubbleLongPreview() {
    ChatPreview {
        TranscriptMessage(
            message("u2", RoleView.USER, MessagePartView.Text(LONG_TEXT)),
            onCopy = {},
            onEdit = {},
        )
    }
}

@Preview(name = "Agent turn, thinking collapsed", showBackground = true)
@Composable
internal fun AgentCollapsedThinkingPreview() {
    ChatPreview {
        TranscriptMessage(
            message(
                "a1",
                RoleView.ASSISTANT,
                MessagePartView.Thinking("Check the session store first."),
                MessagePartView.Text("The retry runs twice on CI."),
            ),
            anchor = TurnAnchor("The retry runs twice on CI.", newest = true),
            onCopy = {},
            onRegenerate = {},
        )
    }
}

@Preview(name = "Thinking, expanded", showBackground = true)
@Composable
internal fun AgentExpandedThinkingPreview() {
    ChatPreview {
        ThinkingBlock(
            stateKey = "preview-open",
            text = "Check the session store first.\n\nThen compare the CI log.",
            live = false,
            startOpen = true,
        )
    }
}

@Preview(name = "Agent turn, tool card", showBackground = true)
@Composable
internal fun AgentToolCardPreview() {
    ChatPreview {
        TranscriptMessage(
            message(
                "a2",
                RoleView.ASSISTANT,
                MessagePartView.ToolCall("call1", "read", "{}"),
                MessagePartView.Text("I read the file."),
            ),
            tools = sample,
        )
    }
}

@Preview(name = "Agent turn, markdown", showBackground = true)
@Composable
internal fun AgentMarkdownPreview() {
    ChatPreview {
        TranscriptMessage(
            message(
                "a3",
                RoleView.ASSISTANT,
                MessagePartView.Text(
                    "## Plan\nOpen `index.html` first.\n- Check **the** test\n- Run *once*\n" +
                        "1. Read the log\n```\nfun main() {}\n```",
                ),
            ),
        )
    }
}

@Preview(
    name = "Narrow, 200 percent font",
    showBackground = true,
    fontScale = LARGE_FONT_SCALE,
    widthDp = NARROW_WIDTH,
)
@Composable
internal fun NarrowLargeFontPreview() {
    ChatPreview(fontScale = LARGE_FONT_SCALE, widthDp = NARROW_WIDTH) {
        TranscriptMessage(
            message("u3", RoleView.USER, MessagePartView.Text(LONG_TEXT)),
            onCopy = {},
            onEdit = {},
        )
        TranscriptMessage(
            message(
                "a4",
                RoleView.ASSISTANT,
                MessagePartView.Thinking("Reasoning kept out of sight."),
                MessagePartView.Text(LONG_TEXT),
            ),
            anchor = TurnAnchor(LONG_TEXT, newest = true),
            onCopy = {},
            onRegenerate = {},
        )
    }
}
