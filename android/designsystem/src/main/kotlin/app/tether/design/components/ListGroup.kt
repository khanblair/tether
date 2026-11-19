package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius

/**
 * A grouped list container: a surface with a hairline border and rounded corners. Children stack
 * with no gap, so the caller adds dividers between rows.
 */
@Composable
fun ListGroup(
    modifier: Modifier = Modifier,
    content: @Composable ColumnScope.() -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
        content = content,
    )
}
