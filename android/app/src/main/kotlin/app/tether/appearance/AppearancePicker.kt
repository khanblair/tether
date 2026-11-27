package app.tether.appearance

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import app.tether.R
import app.tether.design.ACCENT_SWATCHES
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.deriveAccentPalette
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.design.tokens.TetherTokens

private const val HUE_MAX = 360f
private const val FIRST_HUE_STOP = 0
private const val HUE_STOP_COUNT = 6
private const val HUE_STEP = 60f
private const val SATURATION_MAX = 1f
private const val VALUE_MAX = 1f
private const val HALF = 2f

/** The five built-in colors and the custom color entry. */
@Composable
fun AccentSwatchRow(
    selectedId: String?,
    customArgb: Int,
    onSelect: (AccentChoice) -> Unit,
    onOpenCustom: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val choices = accentChoices()
    Row(
        modifier = modifier.horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        choices.forEach { choice ->
            AccentSwatch(
                choice = choice,
                name = swatchName(choice.id),
                selected = choice.id == selectedId,
                onClick = { onSelect(choice) },
            )
        }
        val customSelected = selectedId == null && customArgb != choices.first().argb
        AccentSwatch(
            choice = AccentChoice(CUSTOM_ID, customArgb),
            name = stringResource(R.string.appearance_custom),
            selected = customSelected,
            onClick = onOpenCustom,
        )
    }
}

@Composable
private fun AccentSwatch(
    choice: AccentChoice,
    name: String,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val description =
        stringResource(
            if (selected) {
                R.string.appearance_swatch_selected
            } else {
                R.string.appearance_swatch_not_selected
            },
            name,
        )
    Box(
        modifier =
            Modifier
                .size(TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onClick)
                .semantics {
                    contentDescription = description
                    role = Role.Button
                    this.selected = selected
                },
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier =
                Modifier
                    .size(TetherSpace.space10)
                    .clip(CircleShape)
                    .background(Color(choice.argb), CircleShape)
                    .border(
                        width = if (selected) TetherBorder.inputFocus else TetherBorder.hairline,
                        color =
                            if (selected) {
                                TetherTheme.colors.accentInk
                            } else {
                                TetherTheme.colors.lineInput
                            },
                        shape = CircleShape,
                    ),
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CustomAccentPicker(
    initialArgb: Int,
    onAccentSelected: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    var color by remember(initialArgb) { mutableStateOf(initialArgb) }
    var hsv by remember(initialArgb) { mutableStateOf(argbToHsv(initialArgb)) }
    var hex by remember(initialArgb) { mutableStateOf(formatHexColor(initialArgb)) }
    var error by remember { mutableStateOf<HexError?>(null) }

    ListGroup(modifier = modifier) {
        Column(
            modifier = Modifier.padding(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            Text(
                text = stringResource(R.string.appearance_hue),
                style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
            )
            HueSlider(
                value = hsv.hue,
                onValueChange = { hue ->
                    val selected = hsvToArgb(hue, hsv.saturation, hsv.value)
                    color = selected
                    hsv = argbToHsv(selected)
                    hex = formatHexColor(selected)
                    error = null
                    onAccentSelected(selected)
                },
            )
            Text(
                text = stringResource(R.string.appearance_saturation),
                style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
            )
            Slider(
                value = hsv.saturation,
                onValueChange = { saturation ->
                    val selected = hsvToArgb(hsv.hue, saturation, hsv.value)
                    color = selected
                    hsv = argbToHsv(selected)
                    hex = formatHexColor(selected)
                    error = null
                    onAccentSelected(selected)
                },
                valueRange = 0f..SATURATION_MAX,
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedTextField(
                value = hex,
                onValueChange = { input ->
                    hex = input
                    when (val result = parseHexColor(input)) {
                        is HexColorResult.Valid -> {
                            color = result.argb
                            hsv = argbToHsv(result.argb)
                            error = null
                            onAccentSelected(result.argb)
                        }
                        is HexColorResult.Invalid -> error = result.reason
                    }
                },
                label = { Text(text = stringResource(R.string.appearance_hex)) },
                isError = error != null,
                supportingText = { error?.let { Text(text = stringResource(it.messageRes())) } },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Ascii),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(TetherRadius.sm),
                colors = pickerFieldColors(),
            )
            if (adjustedForReadability(color)) {
                Text(
                    text = stringResource(R.string.appearance_adjusted_for_readability),
                    style = androidx.compose.material3.MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text2,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun HueSlider(
    value: Float,
    onValueChange: (Float) -> Unit,
) {
    val hueDescription = stringResource(R.string.appearance_hue)
    Slider(
        value = value,
        onValueChange = onValueChange,
        valueRange = 0f..HUE_MAX,
        modifier =
            Modifier
                .fillMaxWidth()
                .semantics { contentDescription = hueDescription },
        track = {
            Canvas(
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(TetherSpace.space3),
            ) {
                drawRoundRect(
                    brush = hueBrush(),
                    cornerRadius = CornerRadius(size.height / HALF),
                )
            }
        },
    )
}

@Composable
private fun pickerFieldColors() =
    OutlinedTextFieldDefaults.colors(
        focusedTextColor = TetherTheme.colors.text,
        unfocusedTextColor = TetherTheme.colors.text,
        focusedContainerColor = TetherTheme.colors.surface,
        unfocusedContainerColor = TetherTheme.colors.surface,
        focusedBorderColor = TetherTheme.colors.accent,
        unfocusedBorderColor = TetherTheme.colors.lineInput,
        errorBorderColor = TetherTheme.colors.danger,
        errorSupportingTextColor = TetherTheme.colors.danger,
        cursorColor = TetherTheme.colors.accent,
    )

private fun hueBrush(): Brush =
    Brush.horizontalGradient(
        colors =
            (FIRST_HUE_STOP..HUE_STOP_COUNT).map { index ->
                Color.hsv(index * HUE_STEP, SATURATION_MAX, VALUE_MAX)
            },
    )

@Composable
private fun adjustedForReadability(seed: Int): Boolean {
    val base = if (TetherTheme.isDark) TetherTokens.dark else TetherTokens.light
    val derived = deriveAccentPalette(base, Color(seed), TetherTheme.isDark).color.accent.toArgb()
    return differsVisibly(seed, derived)
}

private fun HexError.messageRes(): Int =
    when (this) {
        HexError.Empty -> R.string.appearance_hex_invalid_empty
        HexError.InvalidLength -> R.string.appearance_hex_invalid_length
        HexError.InvalidCharacters -> R.string.appearance_hex_invalid_characters
    }

@Composable
private fun swatchName(id: String): String =
    stringResource(
        when (id) {
            "plum" -> R.string.appearance_plum
            "blue" -> R.string.appearance_blue
            "green" -> R.string.appearance_green
            "yellow" -> R.string.appearance_yellow
            else -> R.string.appearance_red
        },
    )

private fun accentChoices() =
    ACCENT_SWATCHES.map { swatch -> AccentChoice(swatch.id, swatch.seed.toArgb()) }

private const val CUSTOM_ID = "custom"
