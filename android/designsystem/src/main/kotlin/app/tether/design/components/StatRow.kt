package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * Stats side by side with equal weight, split by hairline dividers. Each stat is one node that
 * TalkBack reads as "label, value, comparison". The comparison is never colored.
 */
@Composable
fun StatRow(
    items: List<StatItem>,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.fillMaxWidth().height(IntrinsicSize.Min),
    ) {
        items.forEachIndexed { index, item ->
            if (index > 0) {
                Box(
                    modifier =
                        Modifier
                            .width(TetherBorder.hairline)
                            .fillMaxHeight()
                            .background(TetherTheme.colors.line),
                )
            }
            StatCell(item = item, modifier = Modifier.weight(1f))
        }
    }
}

@Composable
private fun StatCell(
    item: StatItem,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    Column(
        modifier =
            modifier
                .padding(horizontal = TetherSpace.space3)
                .clearAndSetSemantics {
                    this.contentDescription = "${item.label}, ${item.value}, ${item.comparison}"
                },
    ) {
        Text(
            text = item.value,
            style = MaterialTheme.typography.titleLarge,
            color = colors.text,
        )
        Text(
            text = item.label,
            style = MaterialTheme.typography.labelSmall,
            color = colors.text3,
        )
        Text(
            text = item.comparison,
            style = MaterialTheme.typography.bodySmall,
            color = colors.text2,
        )
    }
}
