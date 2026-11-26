package app.tether.session

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import app.tether.R
import app.tether.core.generated.MessagePartView
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.git.rememberReducedMotion
import app.tether.session.agent.PartSegment

/** The reasoning of one message, as one collapsible block. Collapsed by default. */
@Composable
internal fun ThinkingBlock(
    stateKey: String,
    text: String,
    live: Boolean,
    modifier: Modifier = Modifier,
    startOpen: Boolean = false,
) {
    var open by rememberSaveable(stateKey) { mutableStateOf(startOpen) }
    val reducedMotion = rememberReducedMotion()
    val labelRes = if (live) R.string.chat_thinking_running else R.string.chat_thinking_label
    val label = stringResource(labelRes)
    val stateRes = if (open) R.string.chat_thinking_expanded else R.string.chat_thinking_collapsed
    val state = stringResource(stateRes)
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        ThinkingHeader(label, state, open) { open = !open }
        if (reducedMotion) {
            if (open) ThinkingBody(text)
        } else {
            AnimatedVisibility(visible = open) { ThinkingBody(text) }
        }
    }
}

/** The one row a collapsed block shows. The whole row toggles it. */
@Composable
private fun ThinkingHeader(
    label: String,
    state: String,
    open: Boolean,
    onToggle: () -> Unit,
) {
    val colors = TetherTheme.colors
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onToggle)
                .semantics { stateDescription = state },
    ) {
        TetherIcon(TetherIcons.Brain, contentDescription = null, tint = colors.text2)
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = colors.text2,
            modifier = Modifier.weight(1f),
        )
        val chevron = if (open) TetherIcons.ChevronUp else TetherIcons.ChevronDown
        TetherIcon(chevron, contentDescription = null, tint = colors.text2)
    }
}

/** The reasoning text, with a hairline on its start edge. Not composed while collapsed. */
@Composable
private fun ThinkingBody(text: String) {
    Row(modifier = Modifier.padding(start = TetherSpace.space1).height(IntrinsicSize.Min)) {
        Box(
            Modifier
                .fillMaxHeight()
                .width(TetherBorder.hairline)
                .background(TetherTheme.colors.line),
        )
        Text(
            text = text,
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
            modifier = Modifier.padding(start = TetherSpace.space3),
        )
    }
}

/** True when [segment] is one reasoning part. */
internal fun isThinking(segment: PartSegment): Boolean =
    segment is PartSegment.Part && segment.part is MessagePartView.Thinking

/**
 * Groups [segments] so that consecutive reasoning parts form one run. Every other segment is a
 * run of its own.
 */
internal fun thinkingRuns(segments: List<PartSegment>): List<List<PartSegment>> {
    val runs = mutableListOf<MutableList<PartSegment>>()
    segments.forEach { segment ->
        val previous = runs.lastOrNull()
        if (isThinking(segment) && previous != null && isThinking(previous.last())) {
            previous += segment
        } else {
            runs += mutableListOf(segment)
        }
    }
    return runs
}

/** The reasoning text of one run, one paragraph per part. */
internal fun thinkingText(run: List<PartSegment>): String =
    run
        .mapNotNull { (it as? PartSegment.Part)?.part as? MessagePartView.Thinking }
        .joinToString(separator = "\n\n") { it.text }
