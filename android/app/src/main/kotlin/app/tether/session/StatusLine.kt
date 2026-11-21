package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.home.formatTokens

/**
 * Always visible between the transcript and composer. The model and mode segments open their
 * sheets when [onModelClick] and [onModeClick] are set. A Bypass session gets the danger fill.
 * Usage shows tokens and cost; the context percentage is left out until the core provides it.
 */
@Composable
fun StatusLine(
    status: SessionStatus?,
    usage: SessionUsage?,
    onModelClick: (() -> Unit)?,
    onModeClick: (() -> Unit)?,
) {
    val model = status?.modelId ?: stringResource(R.string.status_unknown)
    val mode = modeLabel(status?.mode)
    val tokens =
        usage?.let {
            stringResource(
                R.string.status_tokens,
                formatTokens(it.tokensIn),
                formatTokens(it.tokensOut),
            )
        } ?: stringResource(R.string.status_unknown)
    val cost =
        usage?.let { costLabel(it.costMicros, stringResource(R.string.session_cost_none)) }
            ?: stringResource(R.string.status_unknown)
    val fill = if (status?.mode == BYPASS_MODE) TetherTheme.colors.dangerSoft else Color.Transparent
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.statusLine)
                .background(fill)
                .padding(horizontal = TetherSpace.space4),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        StatusSegment(model, onModelClick, R.string.session_change_model)
        StatusText(" · ")
        StatusSegment(mode, onModeClick, R.string.session_change_mode)
        StatusText(stringResource(R.string.status_usage, tokens, cost))
    }
}

private const val BYPASS_MODE = "bypass"

/** A tappable part of the line. The touch target is 48 dp; the text keeps its size. */
@Composable
private fun StatusSegment(
    text: String,
    onClick: (() -> Unit)?,
    actionLabel: Int,
) {
    val tap =
        if (onClick == null) {
            Modifier
        } else {
            Modifier
                .minimumInteractiveComponentSize()
                .clickable(
                    role = Role.Button,
                    onClickLabel = stringResource(actionLabel),
                    onClick = onClick,
                )
        }
    StatusText(text, tap)
}

@Composable
private fun StatusText(
    text: String,
    modifier: Modifier = Modifier,
) {
    Text(text = text, color = TetherTheme.colors.text3, modifier = modifier)
}

@Composable
private fun modeLabel(mode: String?): String =
    when (mode) {
        "plan" -> stringResource(R.string.status_mode_plan)
        "ask" -> stringResource(R.string.status_mode_ask)
        "accept_edits" -> stringResource(R.string.status_mode_accept_edits)
        "auto" -> stringResource(R.string.status_mode_auto)
        BYPASS_MODE -> stringResource(R.string.status_mode_bypass)
        else -> stringResource(R.string.status_unknown)
    }
