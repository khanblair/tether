package app.tether.updates

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Density
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

private const val LARGE_FONT_SCALE = 2f

/** A release body as GitHub writes it: a heading, linked bullets, a code span, a footer. */
private val sampleNotes =
    """
    ## What's Changed
    * Show release notes as Markdown by @example-dev in https://github.com/example/tether/pull/12
    * Fix slow update checks by @sample-user in https://github.com/example/tether/pull/14
    * Use tokens on About by @example-dev in https://github.com/example/tether/pull/15

    **Full Changelog**: https://github.com/example/tether/compare/v1.3.0...v1.4.0
    """.trimIndent()

private val sampleState =
    UpdatesUiState(
        currentVersion = "1.3.0",
        installsInApp = false,
        phase = UpdatePhase.Available,
        release =
            ReleaseSummary(
                version = "1.4.0",
                notes = sampleNotes,
                publishedAt = "2026-11-20",
                apkSizeBytes = 42_000_000L,
            ),
    )

@Preview(name = "Release notes, light")
@Composable
internal fun AboutNotesLightPreview() = AboutNotesPreview(ThemeMode.Light, fontScale = null)

@Preview(name = "Release notes, dark")
@Composable
internal fun AboutNotesDarkPreview() = AboutNotesPreview(ThemeMode.Dark, fontScale = null)

@Preview(name = "Release notes, 200% font, light")
@Composable
internal fun AboutNotesLargeFontPreview() =
    AboutNotesPreview(ThemeMode.Light, fontScale = LARGE_FONT_SCALE)

@Composable
private fun AboutNotesPreview(
    mode: ThemeMode,
    fontScale: Float?,
) {
    val density = LocalDensity.current
    val scaled = fontScale?.let { Density(density.density, it) } ?: density
    TetherTheme(mode = mode) {
        CompositionLocalProvider(LocalDensity provides scaled) {
            AboutContent(
                state = sampleState,
                onAction = {},
                onBack = {},
            )
        }
    }
}
