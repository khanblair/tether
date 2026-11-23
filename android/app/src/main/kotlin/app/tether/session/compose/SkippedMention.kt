package app.tether.session.compose

/** A file the person mentioned with `@` that the core did not attach, and why. */
data class SkippedMention(
    val path: String,
    val reason: String,
)

/** The skipped mentions as one line for the snackbar, like "a.bin (not text), b.png (too big)". */
fun skippedMentionsLine(skipped: List<SkippedMention>): String =
    skipped.joinToString(", ") { "${it.path} (${it.reason})" }
