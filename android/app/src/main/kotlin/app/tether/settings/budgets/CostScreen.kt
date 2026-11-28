package app.tether.settings.budgets

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.DataTable
import app.tether.design.components.InlineBanner
import app.tether.design.components.SectionTitle
import app.tether.design.components.SegmentedControl
import app.tether.design.components.StatItem
import app.tether.design.components.StatRow
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.home.formatTokens
import app.tether.settings.SettingsPage

/**
 * The Cost screen: a 7 or 30 day choice, the spend by day, the totals for today and the two
 * windows, and the tables by project and by model. [state] holds the loading, empty, and
 * failure states.
 */
@Composable
fun CostScreen(
    state: CostUiState,
    onSelect: (CostRange) -> Unit,
    onRetry: () -> Unit,
    onBack: () -> Unit,
) {
    SettingsPage(title = stringResource(R.string.budgets_cost_title), onBack = onBack) {
        Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space4)) {
            if (state.failed) {
                InlineBanner(
                    text = stringResource(R.string.budgets_cost_failed),
                    tone = BannerTone.Danger,
                )
                TetherTextButton(
                    label = stringResource(R.string.budgets_try_again),
                    onClick = onRetry,
                )
            }
            RangeChoice(selected = state.range, onSelect = onSelect)
            val summary = state.summary
            when {
                summary != null -> CostBody(state = state, summary = summary)
                !state.failed -> CircularProgressIndicator(color = TetherTheme.colors.text3)
            }
        }
    }
}

/** The 7 and 30 day choice. */
@Composable
private fun RangeChoice(
    selected: CostRange,
    onSelect: (CostRange) -> Unit,
) {
    val week = stringResource(R.string.budgets_range_week)
    val month = stringResource(R.string.budgets_range_month)
    SegmentedControl(
        options = CostRange.entries,
        selected = selected,
        onSelect = onSelect,
        label = { if (it == CostRange.Week) week else month },
    )
}

/** The totals, then the chart and tables, or the empty sentence when there is no usage. */
@Composable
private fun CostBody(
    state: CostUiState,
    summary: CostSummary,
) {
    StatRow(
        items =
            listOf(
                statItem(R.string.budgets_stat_today, summary.today),
                statItem(R.string.budgets_stat_week, summary.last7Days),
                statItem(R.string.budgets_stat_month, summary.last30Days),
            ),
    )
    if (state.empty) {
        Text(
            text = stringResource(R.string.budgets_cost_empty),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        return
    }
    if (state.freeOnly) {
        Text(
            text = stringResource(R.string.budgets_free_only),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
    }
    SectionTitle(
        text =
            stringResource(
                if (state.freeOnly) {
                    R.string.budgets_chart_title_tokens
                } else {
                    R.string.budgets_chart_title_spend
                },
            ),
    )
    CostChart(
        days = summary.byDay,
        metric = if (state.freeOnly) CostMetric.Tokens else CostMetric.Spend,
    )
    CostTables(summary = summary)
}

@Composable
private fun statItem(
    labelRes: Int,
    totals: CostTotals,
): StatItem =
    StatItem(
        value = dollarsText(totals.costMicros),
        label = stringResource(labelRes),
        comparison =
            stringResource(
                R.string.budgets_tokens,
                formatTokens(totals.tokensIn),
                formatTokens(totals.tokensOut),
            ),
    )

/** Cost and tokens by project and by model. A table with no rows is left out. */
@Composable
private fun CostTables(summary: CostSummary) {
    val deleted = stringResource(R.string.budgets_deleted_project)
    val headers =
        stringResource(R.string.budgets_column_project) to
            stringResource(R.string.budgets_column_usage)
    if (summary.byProject.isNotEmpty()) {
        SectionTitle(text = stringResource(R.string.budgets_by_project))
        DataTable(
            headers = headers,
            rows =
                summary.byProject.map {
                    (it.name.ifBlank { deleted }) to
                        usageText(it.costMicros, it.tokensIn, it.tokensOut)
                },
        )
    }
    if (summary.byModel.isNotEmpty()) {
        SectionTitle(text = stringResource(R.string.budgets_by_model))
        DataTable(
            headers =
                stringResource(R.string.budgets_column_model) to
                    stringResource(R.string.budgets_column_usage),
            rows =
                summary.byModel.map {
                    it.modelId to usageText(it.costMicros, it.tokensIn, it.tokensOut)
                },
        )
    }
}

/** The cost and the tokens of one row, such as "$0.00 · 92k in, 1.2k out". */
@Composable
private fun usageText(
    costMicros: Long,
    tokensIn: Long,
    tokensOut: Long,
): String =
    stringResource(
        R.string.budgets_usage_cell,
        dollarsText(costMicros),
        stringResource(R.string.budgets_tokens, formatTokens(tokensIn), formatTokens(tokensOut)),
    )
