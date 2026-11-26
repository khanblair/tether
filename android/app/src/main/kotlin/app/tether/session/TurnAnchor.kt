package app.tether.session

/**
 * The actions under the last reply of one turn. [replyText] is the text of the turn's replies, in
 * order, with thinking and tool calls left out. [newest] is true for the turn that is still the
 * last one in the transcript, the only turn that can regenerate.
 */
internal data class TurnAnchor(
    val replyText: String,
    val newest: Boolean,
)
