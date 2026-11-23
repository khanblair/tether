package app.tether.settings.budgets

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MoneyTest {
    @Test
    fun whole_and_decimal_amounts_parse_to_micros() {
        assertEquals(2_000_000L, parseDollars("2"))
        assertEquals(2_500_000L, parseDollars("2.5"))
        assertEquals(2_500_000L, parseDollars("2.50"))
        assertEquals(10_000L, parseDollars("0.01"))
        assertEquals(2_000_000L, parseDollars("2."))
    }

    @Test
    fun a_comma_or_a_dot_is_the_decimal_mark() {
        assertEquals(2_500_000L, parseDollars("2,5"))
        assertEquals(2_500_000L, parseDollars("2,50"))
    }

    @Test
    fun the_core_range_edges_parse() {
        assertEquals(10_000_000_000L, parseDollars("10000"))
        assertEquals(10_000_000_000L, parseDollars("10000.00"))
    }

    @Test
    fun surrounding_spaces_are_ignored() {
        assertEquals(2_500_000L, parseDollars("  2.50  "))
    }

    @Test
    fun blank_text_is_no_limit_and_parses_to_null() {
        assertNull(parseDollars(""))
        assertNull(parseDollars("   "))
    }

    @Test
    fun invalid_text_parses_to_null() {
        listOf("-1", "0", "0.00", "0.009", "1.234", "abc", "\$2", "1.2.3", "10000.01", "100000")
            .forEach { assertNull(it, parseDollars(it)) }
    }

    @Test
    fun each_bad_amount_names_its_problem() {
        val cases =
            mapOf(
                "-1" to DollarProblem.Negative,
                "0.009" to DollarProblem.TooManyDecimals,
                "1,000.00" to DollarProblem.NotANumber,
                "abc" to DollarProblem.NotANumber,
                "\$2" to DollarProblem.NotANumber,
                "0" to DollarProblem.BelowMinimum,
                "0.005" to DollarProblem.TooManyDecimals,
                "0.00" to DollarProblem.BelowMinimum,
                "10000.01" to DollarProblem.AboveMaximum,
                "100000" to DollarProblem.AboveMaximum,
            )
        cases.forEach { (text, problem) ->
            assertEquals(text, DollarInput.Invalid(problem), readDollars(text))
        }
    }

    @Test
    fun blank_input_reads_as_blank() {
        assertEquals(DollarInput.Blank, readDollars(" "))
    }

    @Test
    fun money_formats_for_people() {
        val cases =
            mapOf(
                0L to "\$0.00",
                -5L to "\$0.00",
                9_999L to "less than \$0.01",
                10_000L to "\$0.01",
                420_000L to "\$0.42",
                424_999L to "\$0.42",
                425_000L to "\$0.43",
                1_250_000_000L to "\$1,250.00",
                10_000_000_000L to "\$10,000.00",
            )
        cases.forEach { (micros, text) ->
            assertEquals(text, formatDollars(micros, "less than \$0.01"))
        }
    }

    @Test
    fun edit_text_has_two_decimals_and_no_symbols() {
        assertEquals("1250.00", editableDollars(1_250_000_000L))
        assertEquals("2.50", editableDollars(2_500_000L))
        assertEquals("0.01", editableDollars(10_000L))
    }

    @Test
    fun edit_text_parses_back_to_the_same_micros() {
        listOf(10_000L, 2_500_000L, 1_250_000_000L, 10_000_000_000L).forEach { micros ->
            assertEquals(micros, parseDollars(editableDollars(micros)))
        }
    }

    @Test
    fun a_missing_limit_gives_blank_edit_text() {
        assertEquals("", editableOrBlank(null))
        assertEquals("2.50", editableOrBlank(2_500_000L))
    }
}
