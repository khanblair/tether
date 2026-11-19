package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSpace

/**
 * One share of a whole: the [label] and [share] text (such as "62%") above a thin bar. The bar
 * fill is [fraction] clamped to 0 to 1, in [color]. TalkBack reads "label, share".
 */
@Composable
fun ShareRow(
    label: String,
    share: String,
    fraction: Float,
    color: Color,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val shape = RoundedCornerShape(TetherRadius.full)
    Column(
        modifier =
            modifier.fillMaxWidth().clearAndSetSemantics {
                this.contentDescription = "$label, $share"
            },
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = label,
                style = MaterialTheme.typography.bodyMedium,
                color = colors.text,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = share,
                style = MaterialTheme.typography.bodySmall,
                color = colors.text2,
            )
        }
        Box(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .height(TetherSpace.space2)
                    .clip(shape)
                    .background(colors.surfaceSunken),
        ) {
            Box(
                modifier =
                    Modifier
                        .fillMaxWidth(fraction.coerceIn(0f, 1f))
                        .height(TetherSpace.space2)
                        .clip(shape)
                        .background(color),
            )
        }
    }
}
