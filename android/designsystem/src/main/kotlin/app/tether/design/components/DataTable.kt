package app.tether.design.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextAlign
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * A two-column table. The header is small and muted, the second column is end-aligned, and a
 * hairline divider sits above each row. Each line is one node for TalkBack.
 */
@Composable
fun DataTable(
    headers: Pair<String, String>,
    rows: List<Pair<String, String>>,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    Column(modifier = modifier.fillMaxWidth()) {
        TableLine(
            first = headers.first,
            second = headers.second,
            style = MaterialTheme.typography.labelSmall,
            color = colors.text3,
        )
        rows.forEach { (label, value) ->
            HorizontalDivider(thickness = TetherBorder.hairline, color = colors.line)
            TableLine(
                first = label,
                second = value,
                style = MaterialTheme.typography.bodySmall,
                color = colors.text,
            )
        }
    }
}

@Composable
private fun TableLine(
    first: String,
    second: String,
    style: TextStyle,
    color: Color,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .semantics(mergeDescendants = true) {}
                .padding(vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(text = first, style = style, color = color, modifier = Modifier.weight(1f))
        Spacer(Modifier.width(TetherSpace.space3))
        Text(text = second, style = style, color = color, textAlign = TextAlign.End)
    }
}
