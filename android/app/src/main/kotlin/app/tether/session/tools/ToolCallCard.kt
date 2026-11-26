package app.tether.session.tools

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.preview.PreviewButton
import app.tether.session.agent.ToolCallUi

/** The name of the run tool. Its calls use [RunCard] instead of the generic card. */
internal const val RUN_TOOL = "run"

/**
 * One tool call as a card: an icon, a title, the status, and an expand control. Expanded, it
 * shows the output in the mono style, selectable and copyable, and scrolls inside the card. A
 * `run` call uses [RunCard]. [expanded] and [onToggle] are hoisted to the caller.
 */
@Composable
fun ToolCallCard(
    call: ToolCallUi,
    expanded: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
    animateRunning: Boolean = false,
    onRunElsewhere: (String) -> Unit = {},
    preview: (() -> Unit)? = null,
) {
    if (call.tool == RUN_TOOL) {
        RunCard(
            call = call,
            expanded = expanded,
            onToggle = onToggle,
            modifier = modifier,
            animateRunning = animateRunning,
            onRunElsewhere = onRunElsewhere,
        )
    } else {
        val title = toolTitle(call)
        ToolCardSurface(modifier = modifier) {
            ToolHeader(
                icon = title.kind.icon(),
                expandable = call.output.isNotEmpty(),
                expanded = expanded,
                onToggle = onToggle,
                trailing = if (preview == null) null else ({ PreviewButton(preview) }),
            ) {
                ToolTitleText(title = title)
                ToolStatusLine(status = call.status, animateRunning = animateRunning)
            }
            if (expanded && call.output.isNotEmpty()) {
                ToolOutputBlock(output = call.output, truncated = call.outputTruncated)
            }
        }
    }
}

/**
 * The output of a call: a note when the start was cut, then the text in a box that scrolls past
 * [TetherSize.notesMax]. The text can be selected and copied.
 */
@Composable
internal fun ToolOutputBlock(
    output: String,
    truncated: Boolean,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    Column(modifier = modifier.toolBodyPadding()) {
        if (truncated) {
            Text(
                text = stringResource(R.string.tools_output_truncated),
                style = MaterialTheme.typography.bodySmall,
                color = colors.text3,
                modifier = Modifier.padding(bottom = TetherSpace.space1),
            )
        }
        OutputText(text = output)
    }
}

/** The scrolling, selectable output box shared by tool cards and run cards. */
@Composable
internal fun OutputText(
    text: String,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    Box(
        modifier =
            modifier
                .fillMaxWidth()
                .heightIn(max = TetherSize.notesMax)
                .background(colors.codeBg, RoundedCornerShape(TetherRadius.sm))
                .verticalScroll(rememberScrollState())
                .padding(TetherSpace.space3),
    ) {
        SelectionContainer {
            Text(text = text, style = TetherTheme.monoSmall, color = colors.text2)
        }
    }
}
