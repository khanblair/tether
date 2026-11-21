package app.tether.memory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.Role
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize

/**
 * A text input for the memory, vault, and skill screens. It is a Material field styled with the
 * tokens until the design system's text field ships. [hint] is shown under the field unless an
 * [error] replaces it.
 */
@Composable
internal fun AreaTextField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    hint: String? = null,
    error: String? = null,
    mono: Boolean = false,
    minLines: Int = 1,
    enabled: Boolean = true,
) {
    val message = error ?: hint
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        enabled = enabled,
        singleLine = minLines == 1,
        minLines = minLines,
        textStyle = if (mono) TetherTheme.mono else MaterialTheme.typography.bodyLarge,
        label = { Text(text = label, style = MaterialTheme.typography.bodySmall) },
        supportingText =
            message?.let {
                {
                    Text(
                        text = it,
                        style = MaterialTheme.typography.bodySmall,
                        color =
                            if (error !=
                                null
                            ) {
                                TetherTheme.colors.danger
                            } else {
                                TetherTheme.colors.text3
                            },
                    )
                }
            },
        isError = error != null,
    )
}

/**
 * A row of tabs with one selected. The selected tab is filled with the tonal color and reports
 * itself as selected to TalkBack. Each tab is at least [TetherSize.touchMin] tall.
 */
@Composable
internal fun AreaTabs(
    labels: List<String>,
    selected: Int,
    onSelect: (Int) -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.sm)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        labels.forEachIndexed { index, label ->
            val isSelected = index == selected
            Box(
                modifier =
                    Modifier
                        .weight(1f)
                        .heightIn(min = TetherSize.touchMin)
                        .background(
                            if (isSelected) {
                                TetherTheme.colors.tonal
                            } else {
                                TetherTheme.colors.surface
                            },
                        ).selectable(
                            selected = isSelected,
                            role = Role.Tab,
                            onClick = { onSelect(index) },
                        ),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = label,
                    style = MaterialTheme.typography.labelLarge,
                    color =
                        if (isSelected) TetherTheme.colors.onTonal else TetherTheme.colors.text2,
                )
            }
        }
    }
}
