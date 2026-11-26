package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.notices.ContextMeter
import app.tether.session.notices.CostMeter

/**
 * The status line parts in the order they show. A Bypass session starts with the Bypass word in
 * place of the mode. The context percentage and the cost show only when they are known.
 */
internal fun statusLineParts(
    bypass: Boolean,
    hasContext: Boolean,
    hasCost: Boolean,
): List<StatusPart> {
    val head =
        if (bypass) {
            listOf(StatusPart.Bypass, StatusPart.Model)
        } else {
            listOf(StatusPart.Model, StatusPart.Mode)
        }
    val tail =
        listOfNotNull(
            StatusPart.Context.takeIf { hasContext },
            StatusPart.Cost.takeIf { hasCost },
        )
    return head + tail
}

/**
 * Always visible between the transcript and composer: model · mode · context · cost. The model
 * shows [modelName] when it is known. The model and mode segments open their sheets, and the cost
 * opens [onOpenCost] when it is set. A Bypass session fills the line with the danger tone and
 * starts with the Bypass word.
 */
@Composable
fun StatusLine(
    status: SessionStatus?,
    usage: SessionUsage?,
    modelName: String?,
    onModelClick: (() -> Unit)?,
    onModeClick: (() -> Unit)?,
    onOpenCost: (() -> Unit)? = null,
) {
    val bypass = status?.mode == BYPASS_MODE
    val percent = status?.contextPercent
    val parts = statusLineParts(bypass, hasContext = percent != null, hasCost = usage != null)
    val fill = if (bypass) TetherTheme.colors.dangerSoft else Color.Transparent
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.statusLine)
                .background(fill)
                .padding(horizontal = TetherSpace.space4),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        parts.forEachIndexed { index, part ->
            if (index > 0) StatusText(" · ", bypass)
            when (part) {
                StatusPart.Bypass ->
                    StatusSegment(
                        stringResource(R.string.status_mode_bypass),
                        onModeClick,
                        R.string.session_change_mode,
                        bypass,
                    )
                StatusPart.Model ->
                    StatusSegment(
                        modelName ?: status?.modelId ?: stringResource(R.string.status_unknown),
                        onModelClick,
                        R.string.session_change_model,
                        bypass,
                    )
                StatusPart.Mode ->
                    StatusSegment(
                        modeLabel(status?.mode),
                        onModeClick,
                        R.string.session_change_mode,
                        bypass,
                    )
                StatusPart.Context ->
                    percent?.let { ContextMeter(percent = it, inDanger = bypass) }
                StatusPart.Cost ->
                    usage?.let {
                        CostMeter(
                            costMicros = it.costMicros,
                            onClick = onOpenCost,
                            inDanger = bypass,
                        )
                    }
            }
        }
    }
}

private const val BYPASS_MODE = "bypass"

/** A tappable part of the line. The touch target is 48 dp; the text keeps its size. */
@Composable
private fun StatusSegment(
    text: String,
    onClick: (() -> Unit)?,
    actionLabel: Int,
    bypass: Boolean,
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
    StatusText(text, bypass, tap)
}

@Composable
private fun StatusText(
    text: String,
    bypass: Boolean,
    modifier: Modifier = Modifier,
) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelSmall,
        color = if (bypass) TetherTheme.colors.danger else TetherTheme.colors.text3,
        fontWeight = if (bypass) FontWeight.Medium else null,
        modifier = modifier,
    )
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
