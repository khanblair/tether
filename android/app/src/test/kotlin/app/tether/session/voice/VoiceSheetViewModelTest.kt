package app.tether.session.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class VoiceSheetViewModelTest {
    private val recognizer = FakeVoiceRecognizer()
    private val model = VoiceSheetViewModel(recognizer)

    private fun listening(): VoiceSheetState.Listening =
        model.state.value as VoiceSheetState.Listening

    @Test fun grantedPermissionStartsListening() {
        model.onPermission(granted = true, canAskAgain = false)

        assertEquals(1, recognizer.starts)
        assertEquals(
            VoiceSheetState.Listening(List(VOICE_BARS) { 0f }, "", active = true),
            model.state.value,
        )
    }

    @Test fun deniedPermissionShowsDeniedAndDoesNotListen() {
        model.onPermission(granted = false, canAskAgain = true)

        assertEquals(VoiceSheetState.Denied(canAskAgain = true), model.state.value)
        assertEquals(0, recognizer.starts)
    }

    @Test fun deniedForeverOffersSettingsNotAsking() {
        model.onPermission(granted = false, canAskAgain = false)

        assertEquals(VoiceSheetState.Denied(canAskAgain = false), model.state.value)
    }

    @Test fun partialTextUpdatesTheTranscript() {
        model.onPermission(granted = true, canAskAgain = false)

        recognizer.sink().onPartial("Fix the log")

        assertEquals("Fix the log", listening().transcript)
        assertTrue(listening().active)
    }

    @Test fun insertIsNullWhileTheTranscriptIsBlank() {
        model.onPermission(granted = true, canAskAgain = false)

        assertNull(model.insert())
    }

    @Test fun finalTextEndsListeningAndInsertReturnsIt() {
        model.onPermission(granted = true, canAskAgain = false)

        recognizer.sink().onFinal("Fix the failing login test")

        assertFalse(listening().active)
        assertEquals("Fix the failing login test", model.insert())
        assertEquals(1, recognizer.cancels)
    }

    @Test fun blankFinalKeepsThePartialText() {
        model.onPermission(granted = true, canAskAgain = false)
        recognizer.sink().onPartial("Run the tests")

        recognizer.sink().onFinal("  ")

        assertEquals("Run the tests", listening().transcript)
    }

    @Test fun cancelDropsTheSpeechAndStopsTheRecognizer() {
        model.onPermission(granted = true, canAskAgain = false)
        recognizer.sink().onPartial("Run the tests")

        model.cancel()

        assertEquals(1, recognizer.cancels)
        assertFalse(listening().active)
    }

    @Test fun lateTextAfterInsertIsIgnored() {
        model.onPermission(granted = true, canAskAgain = false)
        recognizer.sink().onFinal("Run the tests")
        model.insert()

        recognizer.sink().onPartial("late words")

        assertEquals("Run the tests", listening().transcript)
    }

    @Test fun stopEndsListeningThroughTheRecognizerAndKeepsTheFinalText() {
        model.onPermission(granted = true, canAskAgain = false)

        model.stop()
        assertEquals(1, recognizer.stops)
        assertTrue(listening().active)

        recognizer.sink().onFinal("Run the tests")
        assertFalse(listening().active)
        assertEquals("Run the tests", listening().transcript)
    }

    @Test fun levelsShiftInTheNewestLevel() {
        model.onPermission(granted = true, canAskAgain = false)

        recognizer.sink().onLevel(0.5f)
        recognizer.sink().onLevel(0.9f)

        assertEquals(VOICE_BARS, listening().levels.size)
        assertEquals(0.9f, listening().levels.last(), 0f)
        assertEquals(0.5f, listening().levels[VOICE_BARS - 2], 0f)
    }

    @Test fun unavailableRecognizerShowsUnavailable() {
        recognizer.failOnStart = VoiceFailure.Unavailable

        model.onPermission(granted = true, canAskAgain = false)

        assertEquals(VoiceSheetState.Unavailable, model.state.value)
    }

    @Test fun permissionRefusedByTheEngineShowsDenied() {
        recognizer.failOnStart = VoiceFailure.PermissionDenied

        model.onPermission(granted = true, canAskAgain = false)

        assertEquals(VoiceSheetState.Denied(canAskAgain = true), model.state.value)
    }

    @Test fun otherFailuresShowFailedWithTheReason() {
        model.onPermission(granted = true, canAskAgain = false)

        recognizer.sink().onFailure(VoiceFailure.Network)

        assertEquals(VoiceSheetState.Failed(VoiceFailure.Network), model.state.value)
    }

    @Test fun tryAgainListensAgain() {
        model.onPermission(granted = true, canAskAgain = false)
        recognizer.sink().onFailure(VoiceFailure.NoMatch)

        model.retry()

        assertEquals(2, recognizer.starts)
        assertTrue(listening().active)
    }
}
