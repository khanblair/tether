package app.tether.session

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/** Always visible between the transcript and composer. */
@Composable
fun StatusLine(status: SessionStatus?) {
    val model = status?.modelId ?: stringResource(R.string.status_unknown)
    val mode =
        when (status?.mode) {
            "plan" -> stringResource(R.string.status_mode_plan)
            "ask" -> stringResource(R.string.status_mode_ask)
            "accept_edits" -> stringResource(R.string.status_mode_accept_edits)
            "auto" -> stringResource(R.string.status_mode_auto)
            "bypass" -> stringResource(R.string.status_mode_bypass)
            else -> stringResource(R.string.status_unknown)
        }
    val context =
        status?.contextPercent?.let { stringResource(R.string.status_percent, it) }
            ?: stringResource(R.string.status_unknown)
    val cost =
        status?.costText()?.let { stringResource(R.string.status_cost, it) }
            ?: stringResource(R.string.status_unknown)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.statusLine)
                .padding(horizontal = TetherSpace.space4),
    ) {
        Text(
            text = stringResource(R.string.status_line, model, mode, context, cost),
            color = TetherTheme.colors.text3,
        )
    }
}
