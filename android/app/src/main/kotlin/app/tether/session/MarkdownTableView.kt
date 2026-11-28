package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/** A column is at most this share of the transcript width, so a long cell wraps. */
private const val MAX_COLUMN_SHARE = 0.4f

/**
 * A pipe table from an agent reply. Columns are as wide as their widest cell up to a cap, cells
 * wrap, every cell in a row has the row's height, and a table wider than the screen scrolls
 * sideways inside its border. The header row sits on the sunken surface. Borders are the soft
 * line color at hairline width.
 */
@Composable
internal fun MarkdownTableView(
    table: MarkdownTable,
    styles: InlineStyles,
    modifier: Modifier = Modifier,
) {
    val shape = RoundedCornerShape(TetherRadius.sm)
    val columns = table.header.size
    Box(
        modifier =
            modifier
                .clip(shape)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        TableGrid(
            columns = columns,
            maxColumn = TetherSize.transcriptMax * MAX_COLUMN_SHARE,
            modifier = Modifier.horizontalScroll(rememberScrollState()),
        ) {
            table.header.forEachIndexed { column, cell ->
                TableCell(
                    text = cell,
                    header = true,
                    align = table.align[column],
                    edges = CellEdges(column == columns - 1, table.rows.isEmpty()),
                    styles = styles,
                )
            }
            table.rows.forEachIndexed { row, cells ->
                cells.forEachIndexed { column, cell ->
                    TableCell(
                        text = cell,
                        header = false,
                        align = table.align[column],
                        edges =
                            CellEdges(
                                lastColumn = column == columns - 1,
                                lastRow = row == table.rows.lastIndex,
                            ),
                        styles = styles,
                    )
                }
            }
        }
    }
}

/** Which dividers a cell leaves out: none on the last column's right or the last row's bottom. */
private data class CellEdges(
    val lastColumn: Boolean,
    val lastRow: Boolean,
)

/**
 * Lays the cells out row by row, [columns] to a row. Each column is as wide as its widest cell,
 * clamped between a small minimum and [maxColumn], and each row is as tall as its tallest cell.
 */
@Composable
private fun TableGrid(
    columns: Int,
    maxColumn: Dp,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Layout(content = content, modifier = modifier) { measurables, _ ->
        val rows = measurables.chunked(columns)
        val minWidth = TetherSpace.space10.roundToPx()
        val maxWidth = maxColumn.roundToPx().coerceAtLeast(minWidth)
        val widths =
            IntArray(columns) { column ->
                rows.maxOf { row ->
                    row[column].maxIntrinsicWidth(Constraints.Infinity).coerceIn(minWidth, maxWidth)
                }
            }
        val heights =
            rows.map { row ->
                row.mapIndexed { column, cell -> cell.minIntrinsicHeight(widths[column]) }.max()
            }
        val placed =
            rows.mapIndexed { index, row ->
                row.mapIndexed { column, cell ->
                    cell.measure(Constraints.fixed(widths[column], heights[index]))
                }
            }
        layout(widths.sum(), heights.sum()) {
            var y = 0
            placed.forEachIndexed { index, row ->
                var x = 0
                row.forEachIndexed { column, cell ->
                    cell.placeRelative(x, y)
                    x += widths[column]
                }
                y += heights[index]
            }
        }
    }
}

@Composable
private fun TableCell(
    text: String,
    header: Boolean,
    align: ColumnAlign,
    edges: CellEdges,
    styles: InlineStyles,
) {
    val colors = TetherTheme.colors
    val stroke = with(LocalDensity.current) { TetherBorder.hairline.toPx() }
    val style = MaterialTheme.typography.bodySmall
    Box(
        modifier =
            Modifier
                .fillMaxSize()
                .then(if (header) Modifier.background(colors.surfaceSunken) else Modifier)
                .drawBehind {
                    val half = stroke / 2
                    if (!edges.lastColumn) {
                        val x = size.width - half
                        drawLine(colors.line, Offset(x, 0f), Offset(x, size.height), stroke)
                    }
                    if (!edges.lastRow) {
                        val y = size.height - half
                        drawLine(colors.line, Offset(0f, y), Offset(size.width, y), stroke)
                    }
                }.padding(horizontal = TetherSpace.space3, vertical = TetherSpace.space2)
                .then(if (header) Modifier.semantics { heading() } else Modifier),
        contentAlignment = align.alignment(),
    ) {
        Text(
            text = inlineMarkdown(text, styles),
            style = if (header) style.copy(fontWeight = FontWeight.Bold) else style,
            color = colors.text,
            textAlign = align.textAlign(),
        )
    }
}

private fun ColumnAlign.alignment(): Alignment =
    when (this) {
        ColumnAlign.START -> Alignment.CenterStart
        ColumnAlign.CENTER -> Alignment.Center
        ColumnAlign.END -> Alignment.CenterEnd
    }

private fun ColumnAlign.textAlign(): TextAlign =
    when (this) {
        ColumnAlign.START -> TextAlign.Start
        ColumnAlign.CENTER -> TextAlign.Center
        ColumnAlign.END -> TextAlign.End
    }
