package app.tether.session

import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration

/** The span styles for inline Markdown. The defaults need no theme. The screen passes its own. */
internal data class InlineStyles(
    val code: SpanStyle = SpanStyle(fontFamily = FontFamily.Monospace),
    val link: SpanStyle = SpanStyle(textDecoration = TextDecoration.Underline),
    val bold: SpanStyle = SpanStyle(fontWeight = FontWeight.Bold),
    val italic: SpanStyle = SpanStyle(fontStyle = FontStyle.Italic),
    val strike: SpanStyle = SpanStyle(textDecoration = TextDecoration.LineThrough),
)
