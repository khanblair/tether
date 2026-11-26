package app.tether.session.voice

/** Why listening failed, in the terms the voice sheet needs. */
enum class VoiceFailure { NoMatch, Network, Busy, PermissionDenied, Unavailable, Other }

/** Receives what a [VoiceRecognizer] hears. Calls arrive on the main thread. */
interface VoiceListener {
    fun onPartial(text: String)

    fun onFinal(text: String)

    /** The input level, from 0 (silent) to 1 (loud). */
    fun onLevel(level: Float)

    fun onFailure(reason: VoiceFailure)
}

/**
 * Speech input. [start] reports to [listener]. [stop] ends listening and still delivers a final
 * result. [cancel] ends listening and drops what was heard. [release] frees the recognizer.
 */
interface VoiceRecognizer {
    fun start(listener: VoiceListener)

    fun stop()

    fun cancel()

    fun release()
}
