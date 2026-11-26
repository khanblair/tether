package app.tether.session.voice

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace

private val SAMPLE_LEVELS = listOf(0.1f, 0.3f, 0.8f, 0.4f, 1f, 0.6f, 0.2f, 0.5f, 0.7f, 0.1f)
private const val SAMPLE_TRANSCRIPT = "Fix the failing login test and add a regression test"

@Composable
private fun VoicePreview(state: VoiceSheetState) {
    TetherTheme(mode = ThemeMode.Light) {
        Box(
            modifier =
                Modifier
                    .background(TetherTheme.colors.surface)
                    .padding(TetherSpace.space4),
        ) {
            ListeningBody(
                state = state,
                onAllow = {},
                onTryAgain = {},
                onInsert = {},
                onCancel = {},
            )
        }
    }
}

@Preview(name = "Requesting", showBackground = true)
@Composable
internal fun RequestingPreview() = VoicePreview(VoiceSheetState.Requesting)

@Preview(name = "Listening, empty", showBackground = true)
@Composable
internal fun ListeningEmptyPreview() =
    VoicePreview(VoiceSheetState.Listening(SAMPLE_LEVELS, "", active = true))

@Preview(name = "Listening, transcript", showBackground = true)
@Composable
internal fun ListeningTranscriptPreview() =
    VoicePreview(VoiceSheetState.Listening(SAMPLE_LEVELS, SAMPLE_TRANSCRIPT, active = true))

@Preview(name = "Denied, can ask again", showBackground = true)
@Composable
internal fun DeniedAskPreview() = VoicePreview(VoiceSheetState.Denied(canAskAgain = true))

@Preview(name = "Denied, open settings", showBackground = true)
@Composable
internal fun DeniedSettingsPreview() = VoicePreview(VoiceSheetState.Denied(canAskAgain = false))

@Preview(name = "Unavailable", showBackground = true)
@Composable
internal fun UnavailablePreview() = VoicePreview(VoiceSheetState.Unavailable)

@Preview(name = "Failed", showBackground = true)
@Composable
internal fun FailedPreview() = VoicePreview(VoiceSheetState.Failed(VoiceFailure.Network))
