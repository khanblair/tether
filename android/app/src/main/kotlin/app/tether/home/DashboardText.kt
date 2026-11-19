package app.tether.home

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R

/** The one sentence at the top of Home, with its running-session sentence when one applies. */
@Composable
internal fun summaryText(summary: SummaryLine): String =
    when (summary) {
        SummaryLine.Welcome -> stringResource(R.string.dashboard_summary_welcome)
        is SummaryLine.NeedsYou ->
            joinSentences(
                pluralStringResource(
                    R.plurals.dashboard_summary_needs,
                    summary.needs,
                    summary.needs,
                ),
                runningSentence(summary.running),
            )
        is SummaryLine.Calm ->
            joinSentences(
                stringResource(R.string.dashboard_summary_calm),
                runningSentence(summary.running),
            )
    }

@Composable
private fun runningSentence(running: Int): String =
    if (running > 0) {
        pluralStringResource(R.plurals.dashboard_summary_running, running, running)
    } else {
        ""
    }

private fun joinSentences(vararg parts: String): String =
    parts
        .filter {
            it.isNotEmpty()
        }.joinToString(" ")

/** A comparison line such as "2 more than yesterday", where [base] names the earlier period. */
@Composable
internal fun changeText(
    change: Change,
    base: String,
): String =
    when (change) {
        Change.Same -> stringResource(R.string.dashboard_change_same, base)
        Change.NoBaseline -> stringResource(R.string.dashboard_change_no_baseline)
        is Change.MoreCount ->
            pluralStringResource(R.plurals.dashboard_change_more, change.n, change.n, base)
        is Change.LessCount ->
            pluralStringResource(R.plurals.dashboard_change_less, change.n, change.n, base)
        is Change.MorePercent ->
            stringResource(R.string.dashboard_change_percent_more, change.percent, base)
        is Change.LessPercent ->
            stringResource(R.string.dashboard_change_percent_less, change.percent, base)
    }

/** The usage sentence for the week, such as "1.2M tokens this week, 16% more than last week.". */
@Composable
internal fun weekSentence(
    usage: UsageSnapshot,
    change: Change,
): String {
    val tokens = formatTokens(usage.weekTokens)
    return when (change) {
        Change.Same -> stringResource(R.string.dashboard_week_same, tokens)
        is Change.MorePercent ->
            stringResource(R.string.dashboard_week_more, tokens, change.percent)
        is Change.LessPercent ->
            stringResource(R.string.dashboard_week_less, tokens, change.percent)
        else -> stringResource(R.string.dashboard_week_none, tokens)
    }
}
