package app.tether.session.tools

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.Info
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.TriangleAlert
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.FindingSeverity
import app.tether.session.agent.QualityFindingUi
import app.tether.session.agent.QualityUi

/**
 * The result of one quality check: the counts as text, the skipped files, and each finding with
 * its severity, place, and message. Blocking findings come first. Tapping a finding calls
 * [onOpenFinding] with its path and line. The caller omits the card when there are no findings.
 */
@Composable
fun QualityReportCard(
    report: QualityUi,
    onOpenFinding: (path: String, line: Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (report.findings.isEmpty()) return
    val colors = TetherTheme.colors
    val hasBlock = report.block > 0
    val headIcon = if (hasBlock) TetherIcons.CircleX else TetherIcons.TriangleAlert
    val headTint = if (hasBlock) colors.danger else colors.warning

    ToolCardSurface(modifier = modifier) {
        Column(
            modifier =
                Modifier.padding(
                    horizontal = TetherSpace.space3,
                    vertical = TetherSpace.space2,
                ),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space1),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                TetherIcon(
                    icon = headIcon,
                    contentDescription = null,
                    tint = headTint,
                    size = TetherSize.iconS,
                )
                Spacer(Modifier.width(TetherSpace.space2))
                Text(
                    text = countsText(report),
                    style = MaterialTheme.typography.titleSmall,
                    color = colors.text,
                )
            }
            if (report.skipped > 0) {
                Text(
                    text =
                        pluralStringResource(
                            R.plurals.tools_skipped_files,
                            report.skipped,
                            report.skipped,
                        ),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.text2,
                )
            }
        }
        sortFindings(report.findings).forEach { finding ->
            FindingRow(finding = finding, onOpen = onOpenFinding)
        }
    }
}

/** The counts as plain text, such as "2 warnings, 1 blocking". Only non-zero counts show. */
@Composable
private fun countsText(report: QualityUi): String {
    val separator = stringResource(R.string.tools_group_separator)
    return qualityCountParts(report)
        .map { part ->
            when (part.kind) {
                QualityCount.Warn ->
                    pluralStringResource(
                        R.plurals.tools_count_warn,
                        part.count,
                        part.count,
                    )
                QualityCount.Block ->
                    pluralStringResource(
                        R.plurals.tools_count_block,
                        part.count,
                        part.count,
                    )
                QualityCount.Info ->
                    pluralStringResource(
                        R.plurals.tools_count_info,
                        part.count,
                        part.count,
                    )
            }
        }.joinToString(separator)
}

/** One finding: a severity icon and word, the `path:line` place, and the message. */
@Composable
private fun FindingRow(
    finding: QualityFindingUi,
    onOpen: (path: String, line: Int) -> Unit,
) {
    val colors = TetherTheme.colors
    val (icon, tint, word) =
        severityLook(
            finding.severity,
            colors.danger,
            colors.warning,
            colors.text2,
        )
    val location = findingLocation(finding.path, finding.line)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClickLabel = location) {
                    onOpen(finding.path, finding.line)
                }.padding(horizontal = TetherSpace.space3, vertical = TetherSpace.space2),
        verticalAlignment = Alignment.Top,
    ) {
        TetherIcon(icon = icon, contentDescription = null, tint = tint, size = TetherSize.iconS)
        Spacer(Modifier.width(TetherSpace.space2))
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = stringResource(word),
                style = MaterialTheme.typography.labelSmall,
                color = tint,
            )
            Text(
                text = location,
                style = TetherTheme.monoSmall,
                color = colors.text2,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = finding.message,
                style = MaterialTheme.typography.bodySmall,
                color = colors.text,
            )
        }
    }
}

/** The icon, tint, and word of a severity. The word always names the severity. */
private fun severityLook(
    severity: FindingSeverity,
    danger: Color,
    warning: Color,
    neutral: Color,
): Triple<ImageVector, Color, Int> =
    when (severity) {
        FindingSeverity.Block -> Triple(TetherIcons.CircleX, danger, R.string.tools_finding_block)
        FindingSeverity.Warn ->
            Triple(
                TetherIcons.TriangleAlert,
                warning,
                R.string.tools_finding_warn,
            )
        FindingSeverity.Info -> Triple(TetherIcons.Info, neutral, R.string.tools_finding_info)
    }
