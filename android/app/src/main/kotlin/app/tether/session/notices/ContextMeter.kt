package app.tether.session.notices

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
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
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSpace

/**
 * The context use of the session, as a percentage in the status line. The tone follows the
 * [contextBand]: neutral, warning, then danger, and the number is always shown. A tap calls
 * [onOpenBreakdown], which opens the context breakdown.
 */
@Composable
fun ContextMeter(
    percent: Int,
    onOpenBreakdown: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val band = contextBand(percent)
    val tone = bandColor(band)
    val text =
        if (band == ContextBand.Danger) {
            stringResource(R.string.notices_context_danger_label, percent)
        } else {
            stringResource(R.string.notices_context_label, percent)
        }
    val description = stringResource(R.string.notices_context_description, percent)
    Row(
        modifier =
            modifier
                .minimumInteractiveComponentSize()
                .clickable(
                    role = Role.Button,
                    onClickLabel = description,
                    onClick = onOpenBreakdown,
                ).semantics { contentDescription = description }
                .padding(horizontal = TetherSpace.space1),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelSmall,
            color = tone,
        )
    }
}

/** The text color for a context band. Warning and danger text stays readable on the surface. */
@Composable
private fun bandColor(band: ContextBand): Color =
    when (band) {
        ContextBand.Neutral -> TetherTheme.colors.text3
        ContextBand.Warning -> TetherTheme.colors.warning
        ContextBand.Danger -> TetherTheme.colors.danger
    }
