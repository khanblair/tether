package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class MarkdownBlocksTest {
    @Test fun fencedCodeIsSeparateFromProse() {
        assertEquals(
            listOf(
                MarkdownBlock("Before", false),
                MarkdownBlock("val x = 1", true, "kotlin"),
                MarkdownBlock("After", false),
            ),
            splitMarkdown("Before\n```kotlin\nval x = 1\n```\nAfter"),
        )
    }

    @Test fun boldMarkdownRendersWithoutMarkers() {
        assertEquals("A bold word", inlineMarkdown("A **bold** word").text)
        assertEquals("A **broken", inlineMarkdown("A **broken").text)
    }

    @Test fun unfinishedFenceKeepsCodeVisible() {
        assertEquals(
            listOf(MarkdownBlock("line", true)),
            splitMarkdown("```\nline"),
        )
    }

    @Test fun theFenceLanguageIsKept() {
        assertEquals(
            listOf(MarkdownBlock("<p>hi</p>", true, "html")),
            splitMarkdown("```html\n<p>hi</p>\n```"),
        )
    }

    @Test fun aLanguageAfterSpacesIsTheWordAfterTheBackticks() {
        assertEquals(
            listOf(MarkdownBlock("<p>hi</p>", true, "HTML")),
            splitMarkdown("``` HTML\n<p>hi</p>\n```"),
        )
    }

    @Test fun onlyTheFirstWordOfTheInfoStringIsTheLanguage() {
        assertEquals(
            listOf(MarkdownBlock("<p>hi</p>", true, "html")),
            splitMarkdown("```html title=x\n<p>hi</p>\n```"),
        )
    }

    @Test fun aFenceWithNoLanguageHasNoLanguage() {
        assertEquals(listOf(MarkdownBlock("plain", true, "")), splitMarkdown("```\nplain\n```"))
    }

    @Test fun proseAfterAHtmlBlockHasNoLanguage() {
        assertEquals(
            listOf(MarkdownBlock("<b>x</b>", true, "html"), MarkdownBlock("after", false)),
            splitMarkdown("```html\n<b>x</b>\n```\nafter"),
        )
    }
}
