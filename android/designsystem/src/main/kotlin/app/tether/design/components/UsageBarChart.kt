package app.tether.design.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * A bar chart of [values], one bar per value, with a [labels] row under it. The chart is one
 * node for TalkBack, described by [contentDescription].
 */
@Composable
fun UsageBarChart(
    values: List<Long>,
    labels: List<String>,
    contentDescription: String,
    modifier: Modifier = Modifier,
) {
    val chart = TetherTheme.chart
    val fractions = barFractions(values)
    Column(
        modifier =
            modifier.fillMaxWidth().clearAndSetSemantics {
                this.contentDescription = contentDescription
            },
    ) {
        Canvas(modifier = Modifier.fillMaxWidth().height(TetherSize.chartHeight)) {
            drawGridLines(chart.grid)
            drawBars(fractions, chart.series1)
        }
        Row(
            modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space1),
            horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            labels.forEach { label ->
                Text(
                    text = label,
                    style = MaterialTheme.typography.labelSmall,
                    color = chart.axisText,
                    textAlign = TextAlign.Center,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

/** Two hairline grid lines: one at the top of the plot and one at the middle. */
private fun DrawScope.drawGridLines(color: Color) {
    val stroke = TetherBorder.hairline.toPx()
    val top = stroke / 2f
    val middle = size.height / 2f
    drawLine(color, Offset(0f, top), Offset(size.width, top), stroke)
    drawLine(color, Offset(0f, middle), Offset(size.width, middle), stroke)
}

/**
 * Equal-width bars with a gap between them. Each bar rounds its top corners. A zero value
 * still draws a hairline-high bar, so the day stays visible.
 */
private fun DrawScope.drawBars(
    fractions: List<Float>,
    color: Color,
) {
    val count = fractions.size
    if (count == 0) return
    val gap = TetherSpace.space2.toPx()
    val barWidth = ((size.width - gap * (count - 1)) / count).coerceAtLeast(0f)
    val minHeight = TetherBorder.hairline.toPx()
    val maxRadius = TetherRadius.xs.toPx()
    fractions.forEachIndexed { index, fraction ->
        val left = index * (barWidth + gap)
        val height = (fraction * size.height).coerceAtLeast(minHeight)
        val top = size.height - height
        val radius = minOf(maxRadius, barWidth / 2f, height)
        val path =
            Path().apply {
                addRoundRect(
                    RoundRect(
                        rect = Rect(left, top, left + barWidth, size.height),
                        topLeft = CornerRadius(radius),
                        topRight = CornerRadius(radius),
                    ),
                )
            }
        drawPath(path, color)
    }
}
