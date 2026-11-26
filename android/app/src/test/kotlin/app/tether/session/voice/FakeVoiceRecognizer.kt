package app.tether.session.voice

/** A recognizer that records calls. A test sends results through [listener]. */
class FakeVoiceRecognizer : VoiceRecognizer {
    var starts = 0
    var stops = 0
    var cancels = 0
    var releases = 0
    var listener: VoiceListener? = null

    /** When set, [start] reports this failure at once, as the real recognizer does. */
    var failOnStart: VoiceFailure? = null

    override fun start(listener: VoiceListener) {
        starts++
        this.listener = listener
        failOnStart?.let(listener::onFailure)
    }

    override fun stop() {
        stops++
    }

    override fun cancel() {
        cancels++
    }

    override fun release() {
        releases++
    }

    fun sink(): VoiceListener = checkNotNull(listener) { "start was never called" }
}
