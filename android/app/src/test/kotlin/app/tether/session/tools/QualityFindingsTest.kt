package app.tether.session.tools

import app.tether.session.agent.FindingSeverity
import app.tether.session.agent.QualityFindingUi
import app.tether.session.agent.QualityUi
import org.junit.Assert.assertEquals
import org.junit.Test
import java.util.Locale

class QualityFindingsTest {
    private fun finding(
        path: String,
        severity: FindingSeverity,
    ) = QualityFindingUi("rule", severity, path, 1, "message", "advice")

    @Test
    fun blocking_findings_come_first_then_warnings_then_notes() {
        val sorted =
            sortFindings(
                listOf(
                    finding("a", FindingSeverity.Info),
                    finding("b", FindingSeverity.Warn),
                    finding("c", FindingSeverity.Block),
                    finding("d", FindingSeverity.Warn),
                ),
            )
        assertEquals(listOf("c", "b", "d", "a"), sorted.map { it.path })
    }

    @Test
    fun findings_of_one_severity_keep_their_order() {
        val sorted =
            sortFindings(
                listOf(
                    finding("z", FindingSeverity.Warn),
                    finding("y", FindingSeverity.Warn),
                    finding("x", FindingSeverity.Warn),
                ),
            )
        assertEquals(listOf("z", "y", "x"), sorted.map { it.path })
    }

    @Test
    fun the_location_shows_the_line_when_it_is_known() {
        assertEquals("src/auth/session.py:45", findingLocation("src/auth/session.py", 45))
    }

    @Test
    fun the_location_shows_only_the_path_for_a_zero_or_negative_line() {
        assertEquals("CHANGELOG.md", findingLocation("CHANGELOG.md", 0))
        assertEquals("CHANGELOG.md", findingLocation("CHANGELOG.md", -1))
    }

    @Test
    fun the_counts_list_only_the_non_zero_parts_in_reading_order() {
        val report =
            QualityUi(
                "t",
                info = 1,
                warn = 2,
                block = 0,
                findings = emptyList(),
                skipped = 0,
            )
        assertEquals(
            listOf(QualityCount.Warn to 2, QualityCount.Info to 1),
            qualityCountParts(report).map { it.kind to it.count },
        )
    }

    @Test
    fun the_counts_include_blocking_when_present() {
        val report =
            QualityUi(
                "t",
                info = 0,
                warn = 2,
                block = 1,
                findings = emptyList(),
                skipped = 0,
            )
        assertEquals(
            listOf(QualityCount.Warn to 2, QualityCount.Block to 1),
            qualityCountParts(report).map { it.kind to it.count },
        )
    }

    @Test
    fun a_token_count_gets_thousands_separators() {
        assertEquals("41,000", formatTokenCount(41_000, Locale.US))
        assertEquals("6,000", formatTokenCount(6_000, Locale.US))
        assertEquals("999", formatTokenCount(999, Locale.US))
    }
}
