package app.tether.session.voice

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

/** What the voice sheet shows. */
sealed interface VoiceSheetState {
    /** The microphone permission is being asked. */
    data object Requesting : VoiceSheetState

    /**
     * The recognizer is listening, or it has ended ([active] false) with the transcript shown.
     * [levels] holds the last [VOICE_BARS] input levels.
     */
    data class Listening(
        val levels: List<Float>,
        val transcript: String,
        val active: Boolean,
    ) : VoiceSheetState

    /** The microphone permission was refused. [canAskAgain] is false after "don't ask again". */
    data class Denied(
        val canAskAgain: Boolean,
    ) : VoiceSheetState

    /** The device has no speech recognizer. */
    data object Unavailable : VoiceSheetState

    /** Listening stopped with an error the user can retry. */
    data class Failed(
        val reason: VoiceFailure,
    ) : VoiceSheetState
}

/**
 * Drives the voice sheet. The composition creates the recognizer and this class with it, and
 * releases the recognizer when the sheet closes. Callbacks arrive on the main thread, so state is
 * updated at once. Late callbacks after [insert] or [cancel] are ignored.
 */
class VoiceSheetViewModel(
    private val recognizer: VoiceRecognizer,
) : VoiceListener {
    private val mutableState = MutableStateFlow<VoiceSheetState>(VoiceSheetState.Requesting)
    val state: StateFlow<VoiceSheetState> = mutableState.asStateFlow()

    /** The answer to the permission request. A grant starts listening. */
    fun onPermission(
        granted: Boolean,
        canAskAgain: Boolean,
    ) {
        if (granted) begin() else mutableState.value = VoiceSheetState.Denied(canAskAgain)
    }

    /** Listens again after a failure. */
    fun retry() {
        begin()
    }

    /** Stops listening and returns the transcript, or null when it is blank. */
    fun insert(): String? {
        val current = mutableState.value as? VoiceSheetState.Listening ?: return null
        endListening()
        return current.transcript.takeIf(String::isNotBlank)
    }

    /** Stops listening and drops what was heard. */
    fun cancel() {
        endListening()
    }

    /** Ends listening when the app goes to the background. The final result still arrives. */
    fun stop() {
        recognizer.stop()
    }

    override fun onPartial(text: String) {
        updateActive { it.copy(transcript = text) }
    }

    override fun onFinal(text: String) {
        updateActive { it.copy(transcript = text.ifBlank { it.transcript }, active = false) }
    }

    override fun onLevel(level: Float) {
        updateActive { it.copy(levels = nextLevels(it.levels, level)) }
    }

    override fun onFailure(reason: VoiceFailure) {
        val current = mutableState.value
        if (current is VoiceSheetState.Listening && !current.active) return
        mutableState.value =
            when (reason) {
                VoiceFailure.Unavailable -> VoiceSheetState.Unavailable
                VoiceFailure.PermissionDenied -> VoiceSheetState.Denied(canAskAgain = true)
                else -> VoiceSheetState.Failed(reason)
            }
    }

    private fun begin() {
        mutableState.value =
            VoiceSheetState.Listening(
                levels = List(VOICE_BARS) { 0f },
                transcript = "",
                active = true,
            )
        recognizer.start(this)
    }

    private fun endListening() {
        recognizer.cancel()
        mutableState.update { current ->
            if (current is VoiceSheetState.Listening) current.copy(active = false) else current
        }
    }

    private fun updateActive(transform: (VoiceSheetState.Listening) -> VoiceSheetState.Listening) {
        mutableState.update { current ->
            if (current is VoiceSheetState.Listening &&
                current.active
            ) {
                transform(current)
            } else {
                current
            }
        }
    }
}
