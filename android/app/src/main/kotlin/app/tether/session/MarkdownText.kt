package app.tether.session

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSpace

/** Renders one message part: fenced code as a sunken block, prose with bold spans. */
@Composable
internal fun MarkdownBlocks(source: String) {
    splitMarkdown(source).forEach { block ->
        if (block.code) {
            Text(
                text = block.text,
                style = TetherTheme.mono,
                color = TetherTheme.colors.text,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .background(TetherTheme.colors.surfaceSunken)
                        .padding(TetherSpace.space3),
            )
        } else {
            Text(
                text = inlineMarkdown(block.text),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text,
            )
        }
    }
}

internal data class MarkdownBlock(
    val text: String,
    val code: Boolean,
)

/** Keeps fenced code separate from prose; incomplete fences stay visible while streaming. */
internal fun splitMarkdown(source: String): List<MarkdownBlock> {
    val result = mutableListOf<MarkdownBlock>()
    val text = StringBuilder()
    var inCode = false
    source.lineSequence().forEach { line ->
        if (line.startsWith("```")) {
            if (text.isNotEmpty()) result += MarkdownBlock(text.toString().trimEnd('\n'), inCode)
            text.clear()
            inCode = !inCode
        } else {
            text.append(line).append('\n')
        }
    }
    if (text.isNotEmpty()) result += MarkdownBlock(text.toString().trimEnd('\n'), inCode)
    return result
}

/** Styles bold spans while leaving malformed delimiters as plain text. */
internal fun inlineMarkdown(source: String): AnnotatedString =
    buildAnnotatedString {
        var cursor = 0
        while (cursor < source.length) {
            val start = source.indexOf("**", cursor)
            val end = if (start >= 0) source.indexOf("**", start + 2) else -1
            if (end < 0) {
                append(source.substring(cursor))
                break
            }
            append(source.substring(cursor, start))
            pushStyle(SpanStyle(fontWeight = FontWeight.Bold))
            append(source.substring(start + 2, end))
            pop()
            cursor = end + 2
        }
    }
