package app.tether.session.tools

import app.tether.session.agent.FindingSeverity
import app.tether.session.agent.QualityFindingUi
import app.tether.session.agent.QualityUi

/** The counts a quality report line can show. [Warn] comes first, as people read it. */
enum class QualityCount {
    Warn,
    Block,
    Info,
}

/** One count of a quality report. Only counts above zero are listed. */
data class QualityCountPart(
    val kind: QualityCount,
    val count: Int,
)

/** The non-zero counts of [report], in the order warnings, blocking, notes. */
fun qualityCountParts(report: QualityUi): List<QualityCountPart> =
    listOf(
        QualityCountPart(QualityCount.Warn, report.warn),
        QualityCountPart(QualityCount.Block, report.block),
        QualityCountPart(QualityCount.Info, report.info),
    ).filter { it.count > 0 }

private fun severityRank(severity: FindingSeverity): Int =
    when (severity) {
        FindingSeverity.Block -> 0
        FindingSeverity.Warn -> 1
        FindingSeverity.Info -> 2
    }

/**
 * The findings with blocking ones first, then warnings, then notes. Findings of one severity keep
 * the order the core sent them in.
 */
fun sortFindings(findings: List<QualityFindingUi>): List<QualityFindingUi> =
    findings.sortedBy { severityRank(it.severity) }

/** The place of a finding: `path:line`, or only the path when the line is zero or less. */
fun findingLocation(
    path: String,
    line: Int,
): String = if (line > 0) "$path:$line" else path
