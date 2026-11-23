package app.tether.design.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * Two to four exclusive options in one row. The selected option has a tonal fill, a check icon,
 * and its label, so the choice never depends on color alone. Each option is at least 48 dp tall.
 */
@Composable
fun <T> SegmentedControl(
    options: List<T>,
    selected: T,
    onSelect: (T) -> Unit,
    label: (T) -> String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val shape = RoundedCornerShape(TetherRadius.sm)
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .selectableGroup()
                .clip(shape)
                .border(TetherBorder.input, TetherTheme.colors.lineInput, shape),
        horizontalArrangement = Arrangement.Start,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        options.forEach { option ->
            val isSelected = option == selected
            Row(
                modifier =
                    Modifier
                        .weight(1f)
                        .heightIn(min = TetherSize.touchMin)
                        .background(if (isSelected) TetherTheme.colors.tonal else Color.Transparent)
                        .selectable(
                            selected = isSelected,
                            enabled = enabled,
                            role = Role.RadioButton,
                            onClick = { onSelect(option) },
                        ).padding(horizontal = TetherSpace.space2),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (isSelected) {
                    TetherIcon(
                        icon = TetherIcons.CircleCheck,
                        contentDescription = null,
                        tint = TetherTheme.colors.onTonal,
                        modifier = Modifier.size(TetherSize.iconXs),
                    )
                    Spacer(
                        modifier = Modifier.size(TetherSpace.space1),
                    )
                }
                Text(
                    text = label(option),
                    style = MaterialTheme.typography.labelLarge,
                    color =
                        if (isSelected) {
                            TetherTheme.colors.onTonal
                        } else {
                            TetherTheme.colors.text2
                        },
                )
            }
        }
    }
}
