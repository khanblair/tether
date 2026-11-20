package app.tether.design.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius

/**
 * A single-line outlined search field with a leading search icon. A clear button named by
 * [clearLabel] appears only when [value] is not empty.
 */
@Composable
fun TetherSearchField(
    value: String,
    onValueChange: (String) -> Unit,
    placeholder: String,
    clearLabel: String,
    modifier: Modifier = Modifier,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        singleLine = true,
        textStyle = MaterialTheme.typography.bodyLarge,
        placeholder = {
            Text(
                text = placeholder,
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text3,
            )
        },
        leadingIcon = {
            TetherIcon(
                icon = TetherIcons.Search,
                contentDescription = null,
                tint = TetherTheme.colors.text2,
            )
        },
        trailingIcon = {
            if (value.isNotEmpty()) {
                TetherIconButton(
                    icon = TetherIcons.X,
                    contentDescription = clearLabel,
                    onClick = { onValueChange("") },
                )
            }
        },
        keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
        shape = RoundedCornerShape(TetherRadius.sm),
        colors = searchFieldColors(),
    )
}

@Composable
private fun searchFieldColors() =
    OutlinedTextFieldDefaults.colors(
        focusedTextColor = TetherTheme.colors.text,
        unfocusedTextColor = TetherTheme.colors.text,
        focusedContainerColor = TetherTheme.colors.surface,
        unfocusedContainerColor = TetherTheme.colors.surface,
        focusedBorderColor = TetherTheme.colors.accent,
        unfocusedBorderColor = TetherTheme.colors.lineInput,
        cursorColor = TetherTheme.colors.accent,
    )
