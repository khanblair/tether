package app.tether.appearance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Density
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.ACCENT_SWATCHES
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.ChipTone
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.SegmentedControl
import app.tether.design.components.TetherChip
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.di.AppModule
import app.tether.session.SessionStatus
import app.tether.session.SessionUsage
import app.tether.session.StatusLine
import app.tether.settings.SettingsPage

/** The Appearance screen. Every choice is applied to the app as soon as it changes. */
@Composable
fun AppearanceScreen(
    module: AppModule,
    onBack: () -> Unit,
) {
    val mode by module.themeMode.collectAsStateWithLifecycle()
    val appearance by module.appearance.appearance.collectAsStateWithLifecycle()
    AppearanceContent(
        mode = mode,
        appearance = appearance,
        onThemeSelected = module::setThemeMode,
        onInterfaceSelected = module.appearance::setInterfacePercent,
        onTextSelected = module.appearance::setTextPercent,
        onAccentSelected = module.appearance::setAccent,
        onResetAll = module.appearance::reset,
        onBack = onBack,
    )
}

@Composable
private fun AppearanceContent(
    mode: ThemeMode,
    appearance: Appearance,
    onThemeSelected: (ThemeMode) -> Unit,
    onInterfaceSelected: (Int) -> Unit,
    onTextSelected: (Int) -> Unit,
    onAccentSelected: (Int?) -> Unit,
    onResetAll: () -> Unit,
    onBack: () -> Unit,
    customPickerInitiallyOpen: Boolean = false,
) {
    val themeLabels =
        mapOf(
            ThemeMode.System to stringResource(R.string.appearance_theme_system),
            ThemeMode.Light to stringResource(R.string.appearance_theme_light),
            ThemeMode.Dark to stringResource(R.string.appearance_theme_dark),
        )
    var customOpen by remember(appearance.accentArgb) {
        mutableStateOf(
            customPickerInitiallyOpen ||
                (
                    selectedAccentId(appearance.accentArgb, accentChoices()) == null &&
                        appearance.accentArgb != null
                ),
        )
    }
    var customArgb by remember(appearance.accentArgb) {
        mutableStateOf(appearance.accentArgb ?: accentChoices().first().argb)
    }
    SettingsPage(title = stringResource(R.string.appearance_title), onBack = onBack) {
        PreviewCard()
        SectionTitle(text = stringResource(R.string.appearance_theme))
        SegmentedControl(
            options = ThemeMode.entries,
            selected = mode,
            onSelect = onThemeSelected,
            label = { themeLabels.getValue(it) },
        )
        SectionTitle(text = stringResource(R.string.appearance_interface_size))
        SegmentedControl(
            options = interfaceSizeSteps(),
            selected = appearance.interfacePercent,
            onSelect = onInterfaceSelected,
            label = ::sizeStepLabel,
        )
        Hint(text = stringResource(R.string.appearance_interface_hint))
        SectionTitle(text = stringResource(R.string.appearance_text_size))
        ListGroup {
            ListRow(
                title = stringResource(R.string.appearance_follow_system),
                trailing = {
                    Switch(
                        checked = appearance.textPercent == TEXT_FOLLOW_SYSTEM,
                        onCheckedChange = { follows ->
                            onTextSelected(if (follows) TEXT_FOLLOW_SYSTEM else FULL_PERCENT)
                        },
                    )
                },
            )
        }
        SegmentedControl(
            options = manualTextSizeSteps(),
            selected =
                appearance.textPercent.takeUnless { it == TEXT_FOLLOW_SYSTEM } ?: FULL_PERCENT,
            onSelect = onTextSelected,
            label = ::sizeStepLabel,
            enabled = appearance.textPercent != TEXT_FOLLOW_SYSTEM,
        )
        Hint(text = stringResource(R.string.appearance_text_hint))
        SectionTitle(text = stringResource(R.string.appearance_accent_color))
        AccentSwatchRow(
            selectedId =
                appearance.accentArgb?.let {
                    selectedAccentId(it, accentChoices())
                } ?: PLUM_ID,
            customArgb = customArgb,
            onSelect = { choice ->
                customOpen = false
                onAccentSelected(choice.argb)
            },
            onOpenCustom = {
                customOpen = true
                onAccentSelected(customArgb)
            },
        )
        if (customOpen) {
            CustomAccentPicker(
                initialArgb = customArgb,
                onAccentSelected = { selected ->
                    customArgb = selected
                    onAccentSelected(selected)
                },
            )
        }
        ResetActions(onResetPlum = { onAccentSelected(null) }, onResetAll = onResetAll)
    }
}

@Composable
private fun PreviewCard() {
    ListGroup {
        Column(
            modifier = Modifier.padding(TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            Text(
                text = stringResource(R.string.appearance_preview),
                style = MaterialTheme.typography.titleSmall,
            )
            PreviewEditedLine()
            Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
                PrimaryButton(
                    label = stringResource(R.string.appearance_preview_primary),
                    onClick = {},
                    modifier = Modifier.weight(1f),
                )
                Button(
                    onClick = {},
                    modifier = Modifier.weight(1f),
                    colors =
                        ButtonDefaults.buttonColors(
                            containerColor = TetherTheme.colors.tonal,
                            contentColor = TetherTheme.colors.onTonal,
                        ),
                ) {
                    Text(text = stringResource(R.string.appearance_preview_tonal))
                }
            }
            TetherChip(
                label = stringResource(R.string.appearance_preview_chip),
                tone = ChipTone.Accent,
            )
            StatusLine(
                status =
                    SessionStatus(
                        modelId = stringResource(R.string.appearance_preview_model),
                        mode = PREVIEW_MODE,
                        tokensIn = PREVIEW_CONTEXT_PERCENT,
                        tokensOut = 0,
                        costMicros = PREVIEW_COST_MICROS,
                        contextWindow = PREVIEW_CONTEXT_WINDOW,
                    ),
                usage =
                    SessionUsage(
                        tokensIn = PREVIEW_CONTEXT_PERCENT,
                        tokensOut = 0,
                        costMicros = PREVIEW_COST_MICROS,
                    ),
                modelName = stringResource(R.string.appearance_preview_model),
                onModelClick = {},
                onModeClick = {},
                onOpenCost = {},
            )
        }
    }
}

@Composable
private fun PreviewEditedLine() {
    Row {
        Text(
            text = "${stringResource(R.string.appearance_preview_edited)} ",
            style = MaterialTheme.typography.bodyLarge,
        )
        Text(text = stringResource(R.string.appearance_preview_file), style = TetherTheme.monoLabel)
        Text(
            text = " ${stringResource(R.string.appearance_preview_added)}",
            color = TetherTheme.colors.diffAddMark,
        )
        Text(
            text = " ${stringResource(R.string.appearance_preview_removed)}",
            color = TetherTheme.colors.diffDelMark,
        )
    }
}

@Composable
private fun Hint(text: String) {
    Text(text = text, style = MaterialTheme.typography.bodySmall, color = TetherTheme.colors.text3)
}

@Composable
private fun ResetActions(
    onResetPlum: () -> Unit,
    onResetAll: () -> Unit,
) {
    Column {
        TetherTextButton(
            label = stringResource(R.string.appearance_reset_plum),
            onClick = onResetPlum,
        )
        TetherTextButton(
            label = stringResource(R.string.appearance_reset_all),
            onClick = onResetAll,
        )
    }
}

private fun accentChoices() =
    ACCENT_SWATCHES.map { swatch -> AccentChoice(swatch.id, swatch.seed.toArgb()) }

private const val FULL_PERCENT = 100
private const val PLUM_ID = "plum"
private const val PREVIEW_MODE = "accept_edits"
private const val PREVIEW_CONTEXT_PERCENT = 34L
private const val PREVIEW_CONTEXT_WINDOW = 100L
private const val PREVIEW_COST_MICROS = 210_000L

@Preview(showBackground = true)
@Composable
internal fun AppearanceLightPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AppearancePreviewContent()
    }
}

@Preview(showBackground = true, uiMode = android.content.res.Configuration.UI_MODE_NIGHT_YES)
@Composable
internal fun AppearanceDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        AppearancePreviewContent()
    }
}

@Preview(showBackground = true, fontScale = 2f)
@Composable
internal fun AppearanceLargeTextPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CompositionLocalProvider(LocalDensity provides Density(1f, 2f)) {
            AppearancePreviewContent()
        }
    }
}

@Preview(showBackground = true, widthDp = 320)
@Composable
internal fun AppearanceNarrowPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AppearancePreviewContent()
    }
}

@Preview(showBackground = true)
@Composable
internal fun AppearanceCustomPickerPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        AppearancePreviewContent(customPickerInitiallyOpen = true)
    }
}

@Composable
private fun AppearancePreviewContent(customPickerInitiallyOpen: Boolean = false) {
    AppearanceContent(
        mode = ThemeMode.System,
        appearance = Appearance(accentArgb = accentChoices().first().argb),
        onThemeSelected = {},
        onInterfaceSelected = {},
        onTextSelected = {},
        onAccentSelected = {},
        onResetAll = {},
        onBack = {},
        customPickerInitiallyOpen = customPickerInitiallyOpen,
    )
}
