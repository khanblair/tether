package app.tether.session.tools

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.FindingSeverity
import app.tether.session.agent.QualityFindingUi
import app.tether.session.agent.QualityUi

private val warnOnly =
    QualityUi(
        toolCallId = "q-1",
        info = 0,
        warn = 2,
        block = 0,
        findings =
            listOf(
                QualityFindingUi(
                    ruleId = "long-function",
                    severity = FindingSeverity.Warn,
                    path = "src/auth/session.py",
                    line = 45,
                    message = "refresh_session is 58 lines. The limit is 50.",
                    advice = "Split it.",
                ),
                QualityFindingUi(
                    ruleId = "repeats-code",
                    severity = FindingSeverity.Warn,
                    path = "tests/test_login.py",
                    line = 63,
                    message = "The assertion copies the production comparison.",
                    advice = "Use a named constant.",
                ),
            ),
        skipped = 0,
    )

private val blockingAndSkipped =
    QualityUi(
        toolCallId = "q-2",
        info = 1,
        warn = 1,
        block = 1,
        findings =
            listOf(
                QualityFindingUi(
                    ruleId = "note",
                    severity = FindingSeverity.Info,
                    path = "CHANGELOG.md",
                    line = 0,
                    message = "Lines were removed that this fix does not touch.",
                    advice = "Check the diff.",
                ),
                QualityFindingUi(
                    ruleId = "secret",
                    severity = FindingSeverity.Block,
                    path = "config/dev.env",
                    line = 3,
                    message = "A token is written to a tracked file.",
                    advice = "Remove it.",
                ),
                QualityFindingUi(
                    ruleId = "long-function",
                    severity = FindingSeverity.Warn,
                    path = "src/auth/session.py",
                    line = 45,
                    message = "refresh_session is 58 lines. The limit is 50.",
                    advice = "Split it.",
                ),
            ),
        skipped = 2,
    )

private val infoOnly =
    QualityUi(
        toolCallId = "q-3",
        info = 1,
        warn = 0,
        block = 0,
        findings =
            listOf(
                QualityFindingUi(
                    ruleId = "note",
                    severity = FindingSeverity.Info,
                    path = "docs/README.md",
                    line = 12,
                    message = "A heading skips a level.",
                    advice = "Use the next level.",
                ),
            ),
        skipped = 0,
    )

@Preview(name = "Quality report, notes only, dark", showBackground = true)
@Composable
internal fun QualityInfoDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { QualitySample(infoOnly) }
}

@Preview(name = "Quality report, warnings, light", showBackground = true)
@Composable
internal fun QualityWarnLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { QualitySample(warnOnly) }
}

@Preview(name = "Quality report, blocking, dark", showBackground = true)
@Composable
internal fun QualityBlockDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { QualitySample(blockingAndSkipped) }
}

@Composable
private fun QualitySample(report: QualityUi) {
    Column(modifier = Modifier.padding(TetherSpace.space4)) {
        QualityReportCard(report = report, onOpenFinding = { _, _ -> })
    }
}

@Preview(name = "System notes, light", showBackground = true)
@Composable
internal fun SystemNotesLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { SystemNotesSample() }
}

@Preview(name = "System notes, dark", showBackground = true)
@Composable
internal fun SystemNotesDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { SystemNotesSample() }
}

@Composable
private fun SystemNotesSample() {
    Column(
        modifier = Modifier.padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        SystemNote(kind = SystemNoteKind.Compacted(tokensBefore = 41_000, tokensAfter = 6_000))
        SystemNote(kind = SystemNoteKind.Rewound)
        SystemNote(kind = SystemNoteKind.ModeChanged(mode = "Accept edits"))
        SystemNote(kind = SystemNoteKind.ModelChanged(model = "Sonnet 5.5"))
    }
}
