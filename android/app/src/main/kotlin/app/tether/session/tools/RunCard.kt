package app.tether.session.tools

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.ToolCallUi
import app.tether.session.agent.ToolStatus

/**
 * A `run` call: the command in the mono style, the executor badge, and the status. While it
 * runs or has output, the last [RUN_TAIL_LINES] lines show; "Show all" opens every line. A failed
 * or timed-out sandbox run offers "Run elsewhere" through [onRunElsewhere], with the call id.
 */
@Composable
fun RunCard(
    call: ToolCallUi,
    expanded: Boolean,
    onToggle: () -> Unit,
    modifier: Modifier = Modifier,
    animateRunning: Boolean = false,
    onRunElsewhere: (String) -> Unit = {},
) {
    val target = parseRunTarget(call.target)
    ToolCardSurface(modifier = modifier) {
        RunHeader(
            call = call,
            target = target,
            expanded = expanded,
            onToggle = onToggle,
            animateRunning = animateRunning,
        )
        RunOutput(call = call, expanded = expanded, onToggle = onToggle)
        if (canRunElsewhere(call.status, target.executor)) {
            TetherTextButton(
                label = stringResource(R.string.tools_run_elsewhere),
                onClick = { onRunElsewhere(call.id) },
                modifier = Modifier.toolBodyPadding(),
            )
        }
    }
}

/** The command, with its executor badge and status. Tapping it expands or collapses the output. */
@Composable
private fun RunHeader(
    call: ToolCallUi,
    target: RunTarget,
    expanded: Boolean,
    onToggle: () -> Unit,
    animateRunning: Boolean,
) {
    ToolHeader(
        icon = TetherIcons.Terminal,
        expandable = tailLines(call.output).hidden > 0,
        expanded = expanded,
        onToggle = onToggle,
    ) {
        if (toolTitle(call).kind == ToolTitleKind.RunServer) {
            ToolTitleText(title = toolTitle(call))
        } else {
            Text(
                text = target.command,
                style = TetherTheme.monoLabel,
                color = TetherTheme.colors.text,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            ExecutorBadge(executor = target.executor)
            if (target.executor != null) {
                Spacer(Modifier.width(TetherSpace.space2))
            }
            ToolStatusLine(status = call.status, animateRunning = animateRunning)
        }
    }
}

/** The live output: the tail while collapsed, every line while expanded. */
@Composable
private fun RunOutput(
    call: ToolCallUi,
    expanded: Boolean,
    onToggle: () -> Unit,
) {
    val colors = TetherTheme.colors
    if (call.output.isEmpty()) {
        if (call.status == ToolStatus.Running) {
            Text(
                text = stringResource(R.string.tools_output_none),
                style = MaterialTheme.typography.bodySmall,
                color = colors.text3,
                modifier = Modifier.toolBodyPadding(),
            )
        }
        return
    }
    val tail = tailLines(call.output)
    val shown = if (expanded) tailLines(call.output, keep = Int.MAX_VALUE) else tail
    Column(
        modifier = Modifier.toolBodyPadding(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        if (call.outputTruncated) {
            Text(
                text = stringResource(R.string.tools_output_truncated),
                style = MaterialTheme.typography.bodySmall,
                color = colors.text3,
            )
        }
        OutputText(text = shown.lines.joinToString("\n"))
        if (tail.hidden > 0) {
            RunLinesToggle(expanded = expanded, hidden = tail.hidden, onToggle = onToggle)
        }
    }
}

/** "Show all" with the hidden count while collapsed, or "Show less" while expanded. */
@Composable
private fun RunLinesToggle(
    expanded: Boolean,
    hidden: Int,
    onToggle: () -> Unit,
) {
    if (expanded) {
        TetherTextButton(label = stringResource(R.string.tools_show_less), onClick = onToggle)
    } else {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = pluralStringResource(R.plurals.tools_lines_hidden, hidden, hidden),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
            Spacer(Modifier.width(TetherSpace.space2))
            TetherTextButton(label = stringResource(R.string.tools_show_all), onClick = onToggle)
        }
    }
}
