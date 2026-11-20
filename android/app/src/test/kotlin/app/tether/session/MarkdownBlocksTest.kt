package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class MarkdownBlocksTest {
    @Test fun fencedCodeIsSeparateFromProse() {
        assertEquals(
            listOf(
                MarkdownBlock("Before", false),
                MarkdownBlock("val x = 1", true),
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
}
