package app.tether.session.notices

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.tooling.preview.Preview
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.InterruptionUi

/**
 * The block for a turn that the system stopped. It names the step that was running, and offers
 * Resume (the primary action) and Discard. While a call runs, both buttons are off and a progress
 * line says what happens. [running] is the call in progress and [failed] is the call that failed.
 */
@Composable
fun InterruptedBlock(
    interruption: InterruptionUi,
    running: NoticeAction?,
    failed: NoticeAction?,
    onResume: () -> Unit,
    onDiscard: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val colors = TetherTheme.colors
    val fill = if (TetherTheme.isDark) colors.surface else colors.surfaceSunken
    val busy = running != null
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space4)
                .background(fill, RoundedCornerShape(TetherRadius.md))
                .padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Row(verticalAlignment = Alignment.Top) {
            TetherIcon(
                icon = TetherIcons.CirclePause,
                contentDescription = null,
                tint = colors.text2,
                size = TetherSize.iconS,
            )
            Spacer(Modifier.width(TetherSpace.space2))
            Text(
                text = titleWithStep(interruption.step),
                style = MaterialTheme.typography.titleSmall,
                color = colors.text,
            )
        }
        if (running != null) {
            ProgressLine(running)
        }
        if (failed != null) {
            InlineBanner(
                text = stringResource(failureRes(failed)),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.notices_try_again),
                onAction = if (failed == NoticeAction.Resume) onResume else onDiscard,
            )
        }
        Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
            PrimaryButton(
                label = stringResource(R.string.notices_resume),
                onClick = onResume,
                enabled = !busy,
            )
            TetherTextButton(
                label = stringResource(R.string.notices_discard),
                onClick = onDiscard,
                enabled = !busy,
            )
        }
    }
}

/** The title with the step in the mono font, so a command reads as a command. */
@Composable
private fun titleWithStep(step: String) =
    buildAnnotatedString {
        append(stringResource(R.string.notices_interrupted_title))
        append(" ")
        withStyle(SpanStyle(fontFamily = TetherTheme.mono.fontFamily)) {
            append(step)
        }
    }

@Composable
private fun ProgressLine(action: NoticeAction) {
    val text =
        if (action == NoticeAction.Resume) {
            stringResource(R.string.notices_resuming)
        } else {
            stringResource(R.string.notices_discarding)
        }
    Row(verticalAlignment = Alignment.CenterVertically) {
        CircularProgressIndicator(
            modifier = Modifier.size(TetherSize.iconXs),
            color = TetherTheme.colors.text2,
        )
        Spacer(Modifier.width(TetherSpace.space2))
        Text(
            text = text,
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
    }
}

private fun failureRes(action: NoticeAction): Int =
    if (action == NoticeAction.Resume) {
        R.string.notices_resume_failed
    } else {
        R.string.notices_discard_failed
    }

@Preview(name = "Interrupted, light")
@Composable
internal fun InterruptedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        InterruptedBlock(
            interruption = InterruptionUi(step = "npm test", atMs = 0L),
            running = null,
            failed = null,
            onResume = {},
            onDiscard = {},
        )
    }
}

@Preview(name = "Resuming, light")
@Composable
internal fun ResumingPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        InterruptedBlock(
            interruption = InterruptionUi(step = "npm test", atMs = 0L),
            running = NoticeAction.Resume,
            failed = null,
            onResume = {},
            onDiscard = {},
        )
    }
}

@Preview(name = "Resume failed, dark")
@Composable
internal fun ResumeFailedDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        InterruptedBlock(
            interruption = InterruptionUi(step = "pytest -q", atMs = 0L),
            running = null,
            failed = NoticeAction.Resume,
            onResume = {},
            onDiscard = {},
        )
    }
}
