package app.tether.memory

/** One piece of a note body: plain text, or a wiki link such as `[[Auth session design]]`. */
sealed interface BodyPart {
    /** Text with no link in it. */
    data class BodyText(
        val text: String,
    ) : BodyPart

    /**
     * A wiki link. [label] is the alias when the link has one, else the target, with the heading
     * after it. [resolved] is true when the core found a note for the target.
     */
    data class BodyLink(
        val target: String,
        val heading: String?,
        val label: String,
        val resolved: Boolean,
    ) : BodyPart
}
