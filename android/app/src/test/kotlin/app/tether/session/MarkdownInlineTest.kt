package app.tether.session

import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MarkdownInlineTest {
    @Test fun inlineCodeLosesItsBackticksAndGetsAMonoSpan() {
        val result = inlineMarkdown("Open `index.html` now")
        val span = result.spanStyles.single()

        assertEquals("Open index.html now", result.text)
        assertEquals(5, span.start)
        assertEquals(15, span.end)
        assertEquals(FontFamily.Monospace, span.item.fontFamily)
    }

    @Test fun markersInsideCodeStayPlain() {
        val result = inlineMarkdown("`**not bold**`")
        val span = result.spanStyles.single()

        assertEquals("**not bold**", result.text)
        assertNull(span.item.fontWeight)
    }

    @Test fun unclosedBacktickStaysVisible() {
        assertEquals("`open and more", inlineMarkdown("`open and more").text)
    }

    @Test fun emptyCodeSpanStaysVisible() {
        assertEquals("a `` b", inlineMarkdown("a `` b").text)
    }

    @Test fun italicNeedsTextTouchingBothDelimiters() {
        val result = inlineMarkdown("an *em* word")
        val span = result.spanStyles.single()

        assertEquals("an em word", result.text)
        assertEquals(3, span.start)
        assertEquals(5, span.end)
        assertEquals(FontStyle.Italic, span.item.fontStyle)
    }

    @Test fun spacedAsteriskIsArithmeticNotItalic() {
        val result = inlineMarkdown("2 * 3 * 4")

        assertEquals("2 * 3 * 4", result.text)
        assertEquals(0, result.spanStyles.size)
    }

    @Test fun boldInsideTextIsStyled() {
        val result = inlineMarkdown("A **bold** word")
        val span = result.spanStyles.single()

        assertEquals("A bold word", result.text)
        assertEquals(FontWeight.Bold, span.item.fontWeight)
    }

    @Test fun linkShowsOnlyItsTextUnderlined() {
        val result = inlineMarkdown("See [the docs](https://example.com/docs) today")
        val span = result.spanStyles.single()

        assertEquals("See the docs today", result.text)
        assertEquals(TextDecoration.Underline, span.item.textDecoration)
    }

    @Test fun linkWithSpacedUrlStaysPlain() {
        assertEquals("[a](b c)", inlineMarkdown("[a](b c)").text)
    }

    @Test fun unfinishedSyntaxStaysPlain() {
        assertEquals("**bo", inlineMarkdown("**bo").text)
        assertEquals("[text](", inlineMarkdown("[text](").text)
        assertEquals("[text](http", inlineMarkdown("[text](http").text)
    }

    @Test fun codeAndBoldInOneLineBothApply() {
        val result = inlineMarkdown("`a` **b**")

        assertEquals("a b", result.text)
        assertEquals(2, result.spanStyles.size)
    }

    @Test fun customStylesAreUsed() {
        val styles = InlineStyles(code = SpanStyle(fontWeight = FontWeight.Light))
        val result = inlineMarkdown("`x`", styles)
        val span = result.spanStyles.single()

        assertEquals(FontWeight.Light, span.item.fontWeight)
    }
}
