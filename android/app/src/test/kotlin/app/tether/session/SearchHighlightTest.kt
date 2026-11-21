package app.tether.session

import org.junit.Assert.assertEquals
import org.junit.Test

class SearchHighlightTest {
    @Test
    fun a_match_in_the_middle_splits_into_three_runs() {
        assertEquals(
            listOf(
                TextRun("the ", matched = false),
                TextRun("boundary", matched = true),
                TextRun(" second", matched = false),
            ),
            splitMatches("the boundary second", "boundary"),
        )
    }

    @Test
    fun the_match_ignores_case_and_keeps_the_original_text() {
        assertEquals(
            listOf(TextRun("Login", matched = true), TextRun(" page", matched = false)),
            splitMatches("Login page", "LOGIN"),
        )
    }

    @Test
    fun every_occurrence_is_marked() {
        assertEquals(
            listOf(
                TextRun("a", matched = true),
                TextRun(" b ", matched = false),
                TextRun("a", matched = true),
            ),
            splitMatches("a b a", "a"),
        )
    }

    @Test
    fun a_blank_query_marks_nothing() {
        assertEquals(listOf(TextRun("text", matched = false)), splitMatches("text", "  "))
    }

    @Test
    fun a_text_without_the_query_is_one_plain_run() {
        assertEquals(listOf(TextRun("text", matched = false)), splitMatches("text", "zzz"))
    }

    @Test
    fun an_empty_text_gives_no_runs() {
        assertEquals(emptyList<TextRun>(), splitMatches("", "a"))
    }
}
