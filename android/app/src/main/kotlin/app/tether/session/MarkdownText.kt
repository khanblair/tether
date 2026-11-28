package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace
import app.tether.preview.PreviewButton

/** The bullet of each nesting level, so a nested list reads as nested. */
private val BULLET_MARKS = listOf("•", "◦", "▪", "▪")

/** The fence language that makes a code block a page, which the Preview button opens. */
private const val HTML_LANGUAGE = "html"

/**
 * Renders one message part: fenced code as a sunken block, prose with inline code, emphasis,
 * links, headings and lists. Thinking and tool output never pass through here. [onPreviewHtml]
 * adds a Preview button under each HTML code block; null adds none.
 */
@Composable
internal fun MarkdownBlocks(
    source: String,
    onPreviewHtml: ((String) -> Unit)? = null,
) {
    val colors = TetherTheme.colors
    val styles =
        InlineStyles(
            code =
                SpanStyle(
                    fontFamily = TetherTheme.mono.fontFamily,
                    background = colors.surfaceSunken,
                ),
            link = SpanStyle(textDecoration = TextDecoration.Underline),
        )
    splitMarkdown(source).forEach { block ->
        if (block.code) {
            CodeBlock(block.text)
            if (onPreviewHtml != null && block.lang.equals(HTML_LANGUAGE, ignoreCase = true)) {
                PreviewButton(onClick = { onPreviewHtml(block.text) })
            }
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
                markdownLines(block.text).forEach { line -> MarkdownLineView(line, styles) }
            }
        }
    }
}

@Composable
private fun CodeBlock(text: String) {
    Text(
        text = text,
        style = TetherTheme.mono,
        color = TetherTheme.colors.text,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(TetherTheme.colors.surfaceSunken)
                .padding(TetherSpace.space3),
    )
}

@Composable
private fun MarkdownLineView(
    line: MarkdownLine,
    styles: InlineStyles,
) {
    val text = inlineMarkdown(line.text, styles)
    when (line.kind) {
        LineKind.PLAIN -> Text(text, style = bodyStyle(), color = TetherTheme.colors.text)
        LineKind.HEADING_1 -> HeadingText(text, MaterialTheme.typography.titleMedium)
        LineKind.HEADING_2 -> HeadingText(text, MaterialTheme.typography.titleSmall)
        LineKind.HEADING_3 -> HeadingText(text, bodyStyle())
        LineKind.BULLET -> ListItem(BULLET_MARKS[line.level], text, line.level)
        LineKind.NUMBERED -> ListItem(line.marker, text, line.level)
        LineKind.QUOTE -> QuoteText(text)
        LineKind.RULE -> RuleLine()
        LineKind.TABLE -> line.table?.let { MarkdownTableView(it, styles) }
    }
}

@Composable
private fun bodyStyle(): TextStyle = MaterialTheme.typography.bodyMedium

/** A heading: the theme's text style, in bold. */
@Composable
private fun HeadingText(
    text: AnnotatedString,
    base: TextStyle,
) {
    Text(
        text,
        style = base.copy(fontWeight = FontWeight.Bold),
        color = TetherTheme.colors.text,
    )
}

/**
 * A list item: the marker, then the text. Wrapped lines stay under the text, not the marker. A
 * nested item is indented one step per [level].
 */
@Composable
private fun ListItem(
    marker: String,
    text: AnnotatedString,
    level: Int,
) {
    val style = bodyStyle()
    val color = TetherTheme.colors.text
    Row(
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        modifier = Modifier.padding(start = TetherSpace.space4 * level),
    ) {
        Text(marker, style = style, color = color)
        Text(text, style = style, color = color)
    }
}

/** A quote: a soft bar at the start and the text in the secondary color. */
@Composable
private fun QuoteText(text: AnnotatedString) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        modifier = Modifier.height(IntrinsicSize.Min),
    ) {
        Box(
            modifier =
                Modifier
                    .fillMaxHeight()
                    .width(TetherSpace.space1)
                    .background(TetherTheme.colors.line),
        )
        Text(text, style = bodyStyle(), color = TetherTheme.colors.text2)
    }
}

/** A horizontal rule: one hairline across the message. */
@Composable
private fun RuleLine() {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(TetherBorder.hairline)
                .background(TetherTheme.colors.line),
    )
}

/** One run of prose or code. [lang] is the fence's language, such as `html`, or empty. */
internal data class MarkdownBlock(
    val text: String,
    val code: Boolean,
    val lang: String = "",
)

/** Keeps fenced code separate from prose; incomplete fences stay visible while streaming. */
internal fun splitMarkdown(source: String): List<MarkdownBlock> {
    val result = mutableListOf<MarkdownBlock>()
    val text = StringBuilder()
    var inCode = false
    var lang = ""

    fun flush() {
        if (text.isNotEmpty()) {
            result += MarkdownBlock(text.toString().trimEnd('\n'), inCode, codeLang(inCode, lang))
        }
        text.clear()
    }
    source.lineSequence().forEach { line ->
        if (line.startsWith("```")) {
            flush()
            if (!inCode) lang = fenceLanguage(line)
            inCode = !inCode
        } else {
            text.append(line).append('\n')
        }
    }
    flush()
    return result
}

/** The language of a block: the fence's language for code, and empty for prose. */
private fun codeLang(
    inCode: Boolean,
    lang: String,
): String = if (inCode) lang else ""

/** The language on an opening fence: the first word after the backticks, such as `html`. */
private fun fenceLanguage(line: String): String =
    line.removePrefix("```").trim().takeWhile { !it.isWhitespace() }
