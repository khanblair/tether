package app.tether.design.components

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize

/**
 * An outlined single-line field for a secret such as an API key. The value is masked unless
 * [revealed]. The field never logs or echoes the value. [error] replaces [hint] when set. A null
 * [onToggleReveal] hides the eye button, for a field with nothing to show.
 */
@Composable
fun SecretField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    revealed: Boolean,
    onToggleReveal: (() -> Unit)?,
    revealLabel: String,
    hideLabel: String,
    pasteLabel: String,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    hint: String? = null,
    error: String? = null,
    enabled: Boolean = true,
    onPaste: (() -> Unit)? = null,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = modifier.fillMaxWidth(),
        enabled = enabled,
        singleLine = true,
        textStyle = TetherTheme.mono,
        label = { Text(text = label, style = MaterialTheme.typography.bodySmall) },
        placeholder = {
            if (placeholder.isNotEmpty()) {
                Text(
                    text = placeholder,
                    style = TetherTheme.mono,
                    color = TetherTheme.colors.text3,
                )
            }
        },
        supportingText = {
            val message = error ?: hint
            val messageColor =
                if (error != null) TetherTheme.colors.danger else TetherTheme.colors.text3
            if (message != null) {
                Text(
                    text = message,
                    style = MaterialTheme.typography.bodySmall,
                    color = messageColor,
                )
            }
        },
        isError = error != null,
        trailingIcon = {
            SecretTrailing(
                revealed = revealed,
                onToggleReveal = onToggleReveal,
                revealLabel = revealLabel,
                hideLabel = hideLabel,
                pasteLabel = pasteLabel,
                onPaste = onPaste,
                enabled = enabled,
            )
        },
        visualTransformation =
            if (revealed) VisualTransformation.None else PasswordVisualTransformation(),
        keyboardOptions =
            KeyboardOptions(
                keyboardType = KeyboardType.Password,
                autoCorrectEnabled = false,
                imeAction = ImeAction.Done,
            ),
        shape = RoundedCornerShape(TetherRadius.sm),
        colors = secretFieldColors(),
    )
}

@Composable
private fun SecretTrailing(
    revealed: Boolean,
    onToggleReveal: (() -> Unit)?,
    revealLabel: String,
    hideLabel: String,
    pasteLabel: String,
    onPaste: (() -> Unit)?,
    enabled: Boolean,
) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        if (onPaste != null) {
            TetherTextButton(label = pasteLabel, onClick = onPaste, enabled = enabled)
        }
        if (onToggleReveal != null) {
            IconButton(
                onClick = onToggleReveal,
                enabled = enabled,
                modifier = Modifier.size(TetherSize.touchMin),
            ) {
                TetherIcon(
                    icon = if (revealed) TetherIcons.EyeOff else TetherIcons.Eye,
                    contentDescription = if (revealed) hideLabel else revealLabel,
                    tint = TetherTheme.colors.text2,
                )
            }
        }
    }
}

@Composable
private fun secretFieldColors() =
    OutlinedTextFieldDefaults.colors(
        focusedTextColor = TetherTheme.colors.text,
        unfocusedTextColor = TetherTheme.colors.text,
        disabledTextColor = TetherTheme.colors.textDisabled,
        focusedContainerColor = TetherTheme.colors.surface,
        unfocusedContainerColor = TetherTheme.colors.surface,
        disabledContainerColor = TetherTheme.colors.surfaceSunken,
        focusedBorderColor = TetherTheme.colors.accent,
        unfocusedBorderColor = TetherTheme.colors.lineInput,
        disabledBorderColor = TetherTheme.colors.line,
        errorBorderColor = TetherTheme.colors.danger,
        focusedLabelColor = TetherTheme.colors.text2,
        unfocusedLabelColor = TetherTheme.colors.text3,
        errorLabelColor = TetherTheme.colors.danger,
        cursorColor = TetherTheme.colors.accent,
        errorCursorColor = TetherTheme.colors.danger,
    )
