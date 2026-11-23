package app.tether.session.tools

import java.util.Locale

/** A quiet marker in the transcript. The composable turns each kind into its text. */
sealed interface SystemNoteKind {
    /** The context was summarized: [tokensBefore] tokens became [tokensAfter]. */
    data class Compacted(
        val tokensBefore: Int,
        val tokensAfter: Int,
    ) : SystemNoteKind

    /** The session was rewound to a checkpoint. */
    data object Rewound : SystemNoteKind

    /** The permission mode changed. [mode] is its display name. */
    data class ModeChanged(
        val mode: String,
    ) : SystemNoteKind

    /** The model changed. [model] is its display name. */
    data class ModelChanged(
        val model: String,
    ) : SystemNoteKind
}

/** A token count with thousands separators, such as "41,000". The locale sets the separator. */
fun formatTokenCount(
    tokens: Int,
    locale: Locale,
): String = String.format(locale, "%,d", tokens)
