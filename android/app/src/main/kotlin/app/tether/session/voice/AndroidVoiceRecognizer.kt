package app.tether.session.voice

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer

/**
 * The device speech recognizer. It must be created and used on the main thread. The recognizer
 * app does the work; Tether keeps no audio.
 */
class AndroidVoiceRecognizer(
    context: Context,
) : VoiceRecognizer {
    private val appContext = context.applicationContext
    private var speech: SpeechRecognizer? = null
    private var listener: VoiceListener? = null

    override fun start(listener: VoiceListener) {
        this.listener = listener
        if (!isAvailable(appContext)) {
            listener.onFailure(VoiceFailure.Unavailable)
            return
        }
        val recognizer = speech ?: createRecognizer().also { speech = it }
        try {
            recognizer.startListening(recognizerIntent())
        } catch (_: SecurityException) {
            listener.onFailure(VoiceFailure.PermissionDenied)
        }
    }

    override fun stop() {
        speech?.stopListening()
    }

    override fun cancel() {
        speech?.cancel()
    }

    override fun release() {
        speech?.destroy()
        speech = null
        listener = null
    }

    private fun createRecognizer(): SpeechRecognizer =
        SpeechRecognizer.createSpeechRecognizer(appContext).apply {
            setRecognitionListener(Bridge())
        }

    /** Forwards the platform callbacks to the listener in the neutral terms of this app. */
    private inner class Bridge : RecognitionListener {
        override fun onReadyForSpeech(params: Bundle?) = Unit

        override fun onBeginningOfSpeech() = Unit

        override fun onRmsChanged(rmsdB: Float) {
            listener?.onLevel(levelFromRms(rmsdB))
        }

        override fun onBufferReceived(buffer: ByteArray?) = Unit

        override fun onEndOfSpeech() = Unit

        override fun onError(error: Int) {
            listener?.onFailure(voiceFailureFor(error))
        }

        override fun onResults(results: Bundle?) {
            listener?.onFinal(firstResult(results))
        }

        override fun onPartialResults(partialResults: Bundle?) {
            listener?.onPartial(firstResult(partialResults))
        }

        override fun onEvent(
            eventType: Int,
            params: Bundle?,
        ) = Unit
    }

    companion object {
        /** True when the device has a speech recognizer. */
        fun isAvailable(context: Context): Boolean =
            SpeechRecognizer.isRecognitionAvailable(context)
    }
}

private fun recognizerIntent(): Intent =
    Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
        putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
        putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
    }

private fun firstResult(bundle: Bundle?): String =
    bundle?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull().orEmpty()

/** Maps a [SpeechRecognizer] error code to the failure the sheet shows. */
internal fun voiceFailureFor(code: Int): VoiceFailure =
    when (code) {
        SpeechRecognizer.ERROR_NO_MATCH,
        SpeechRecognizer.ERROR_SPEECH_TIMEOUT,
        -> VoiceFailure.NoMatch
        SpeechRecognizer.ERROR_NETWORK,
        SpeechRecognizer.ERROR_NETWORK_TIMEOUT,
        SpeechRecognizer.ERROR_SERVER,
        -> VoiceFailure.Network
        SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> VoiceFailure.Busy
        SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> VoiceFailure.PermissionDenied
        else -> VoiceFailure.Other
    }
