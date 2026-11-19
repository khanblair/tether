package app.tether.home

import org.junit.Assert.assertEquals
import org.junit.Test
import java.time.LocalDate
import java.time.format.TextStyle
import java.util.Locale

class UsageFormatTest {
    @Test
    fun token_counts_use_the_short_form_at_each_boundary() {
        assertEquals("0", formatTokens(0L))
        assertEquals("999", formatTokens(999L))
        assertEquals("1k", formatTokens(1_000L))
        assertEquals("1.2k", formatTokens(1_234L))
        assertEquals("10k", formatTokens(9_999L))
        assertEquals("10k", formatTokens(10_000L))
        assertEquals("212k", formatTokens(212_000L))
        assertEquals("999k", formatTokens(999_499L))
        assertEquals("1M", formatTokens(999_500L))
        assertEquals("1M", formatTokens(1_000_000L))
        assertEquals("1.2M", formatTokens(1_234_567L))
    }

    @Test
    fun negative_token_counts_count_as_zero() {
        assertEquals("0", formatTokens(-5L))
    }

    @Test
    fun a_count_change_is_same_more_or_less_by_the_difference() {
        assertEquals(Change.Same, countChange(3, 3))
        assertEquals(Change.MoreCount(2), countChange(4, 2))
        assertEquals(Change.LessCount(2), countChange(2, 4))
    }

    @Test
    fun a_percent_change_rounds_up_and_down() {
        assertEquals(Change.MorePercent(18), percentChange(212_000L, 180_000L))
        assertEquals(Change.MorePercent(17), percentChange(1_166L, 1_000L))
        assertEquals(Change.MorePercent(16), percentChange(1_164L, 1_000L))
        assertEquals(Change.LessPercent(20), percentChange(800L, 1_000L))
    }

    @Test
    fun a_percent_change_that_rounds_to_zero_is_same() {
        assertEquals(Change.Same, percentChange(996L, 1_000L))
        assertEquals(Change.Same, percentChange(0L, 0L))
    }

    @Test
    fun a_percent_change_with_no_earlier_usage_has_no_baseline() {
        assertEquals(Change.NoBaseline, percentChange(5L, 0L))
    }

    @Test
    fun weekday_labels_end_with_today_and_run_oldest_first() {
        val today = LocalDate.of(2026, 11, 19)
        assertEquals(
            listOf("F", "S", "S", "M", "T", "W", "T"),
            weekdayLabels(today, Locale.US, TextStyle.NARROW),
        )
        assertEquals(
            listOf("Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday", "Thursday"),
            weekdayLabels(today, Locale.US, TextStyle.FULL),
        )
    }

    @Test
    fun model_shares_sort_by_tokens_and_round_the_percent() {
        val shares =
            modelShares(
                listOf(
                    ModelUsage("Haiku 5.5", 289_000L),
                    ModelUsage("Sonnet 5.5", 717_000L),
                    ModelUsage("Opus 5.5", 151_000L),
                ),
            )
        assertEquals(listOf("Sonnet 5.5", "Haiku 5.5", "Opus 5.5"), shares.map { it.model })
        assertEquals(listOf(62, 25, 13), shares.map { it.percent })
        assertEquals(0.6197f, shares[0].fraction, 0.0001f)
    }

    @Test
    fun model_shares_drop_zero_models_and_break_ties_by_name() {
        val shares =
            modelShares(
                listOf(
                    ModelUsage("b", 5L),
                    ModelUsage("zero", 0L),
                    ModelUsage("a", 5L),
                ),
            )
        assertEquals(listOf("a", "b"), shares.map { it.model })
    }

    @Test
    fun model_shares_are_empty_for_an_empty_list_or_a_zero_total() {
        assertEquals(emptyList<ModelShare>(), modelShares(emptyList()))
        assertEquals(emptyList<ModelShare>(), modelShares(listOf(ModelUsage("a", 0L))))
    }
}
