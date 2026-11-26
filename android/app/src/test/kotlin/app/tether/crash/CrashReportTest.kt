package app.tether.crash

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Instant

private const val MESSAGE_LINE = 3

class CrashReportTest {
    private val now = Instant.parse("2026-11-26T10:00:00Z")

    @Test
    fun the_first_line_names_the_exception_and_the_time() {
        val report = formatCrashReport(now, "1.2.3", "main", IllegalStateException("boom"))

        assertEquals(
            "java.lang.IllegalStateException at 2026-11-26T10:00:00Z",
            report.lines().first(),
        )
    }

    @Test
    fun the_header_holds_the_version_and_the_thread() {
        val report = formatCrashReport(now, "1.2.3", "main", IllegalStateException("boom"))

        assertTrue(report.contains("Version: 1.2.3\n"))
        assertTrue(report.contains("Thread: main\n"))
        assertTrue(report.contains("Message: boom\n"))
    }

    @Test
    fun a_long_message_is_cut_to_200_characters_in_the_header() {
        val long = "a".repeat(250)

        val report = formatCrashReport(now, "1", "main", IllegalStateException(long))

        assertEquals("Message: ${"a".repeat(200)}", report.lines()[MESSAGE_LINE])
    }

    @Test
    fun the_cut_never_splits_a_surrogate_pair() {
        val message = "a".repeat(199) + "😀"

        val cut = cutMessage(message)

        assertEquals("a".repeat(199), cut)
    }

    @Test
    fun a_null_message_becomes_empty() {
        assertEquals("", cutMessage(null))
    }

    @Test
    fun the_stack_trace_includes_the_cause_chain() {
        val error = IllegalStateException("outer", IllegalArgumentException("inner"))

        val report = formatCrashReport(now, "1", "main", error)

        assertTrue(report.contains("Caused by: java.lang.IllegalArgumentException: inner"))
    }
}
