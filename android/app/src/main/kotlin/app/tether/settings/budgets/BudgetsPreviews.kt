@file:Suppress("MagicNumber")

package app.tether.settings.budgets

import androidx.compose.runtime.Composable
import androidx.compose.ui.tooling.preview.Preview
import app.tether.core.generated.ProviderView
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode

private const val DAILY_MICROS = 2_000_000L
private const val SESSION_MICROS = 500_000L

@Preview(name = "Budgets, set, light", showBackground = true)
@Composable
internal fun BudgetsSetLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { BudgetsSample(form = savedForm()) }
}

@Preview(name = "Budgets, set, dark", showBackground = true)
@Composable
internal fun BudgetsSetDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { BudgetsSample(form = savedForm()) }
}

@Preview(name = "Budgets, no limit", showBackground = true)
@Composable
internal fun BudgetsNoLimitPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetsSample(form = BudgetsUiState(loading = false))
    }
}

@Preview(name = "Budgets, invalid amount", showBackground = true)
@Composable
internal fun BudgetsInvalidPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetsSample(form = savedForm().copy(daily = "2.505", saveFailure = null))
    }
}

@Preview(name = "Budgets, saved", showBackground = true)
@Composable
internal fun BudgetsSavedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetsSample(form = savedForm().copy(saved = true))
    }
}

@Preview(name = "Budgets, save failed", showBackground = true)
@Composable
internal fun BudgetsSaveFailedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetsSample(
            form = savedForm().copy(daily = "3", saveFailure = BudgetsProblem.Unavailable),
        )
    }
}

@Preview(name = "Budgets, load failed", showBackground = true)
@Composable
internal fun BudgetsLoadFailedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetsSample(form = BudgetsUiState(loading = false, loadFailed = true))
    }
}

@Preview(name = "Cost, spend, light", showBackground = true)
@Composable
internal fun CostSpendLightPreview() {
    TetherTheme(mode = ThemeMode.Light) { CostSample(state = costState()) }
}

@Preview(name = "Cost, spend, dark", showBackground = true)
@Composable
internal fun CostSpendDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) { CostSample(state = costState()) }
}

@Preview(name = "Cost, no spend yet", showBackground = true)
@Composable
internal fun CostEmptyPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CostSample(state = costState(summary = sampleSummary(spend = false)))
    }
}

@Preview(name = "Cost, failed", showBackground = true)
@Composable
internal fun CostFailedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        CostSample(state = CostUiState(loading = false, failed = true))
    }
}

private fun savedForm(): BudgetsUiState =
    BudgetsUiState(
        loading = false,
        daily = editableDollars(DAILY_MICROS),
        session = editableDollars(SESSION_MICROS),
        savedDaily = DAILY_MICROS,
        savedSession = SESSION_MICROS,
    )

@Composable
private fun BudgetsSample(form: BudgetsUiState) {
    BudgetsScreen(
        state = form,
        onDailyChange = {},
        onSessionChange = {},
        onRemoveDaily = {},
        onRemoveSession = {},
        onSave = {},
        onRetry = {},
        onBack = {},
    )
}

@Composable
private fun CostSample(state: CostUiState) {
    CostScreen(state = state, onSelect = {}, onRetry = {}, onBack = {})
}

private fun costState(summary: CostSummary = sampleSummary(spend = true)): CostUiState =
    CostUiState(loading = false, summary = summary)

private fun sampleSummary(spend: Boolean): CostSummary {
    val costs =
        if (spend) {
            listOf(
                400_000L,
                900_000L,
                0L,
                1_400_000L,
                700_000L,
                300_000L,
                840_000L,
            )
        } else {
            List(7) {
                0L
            }
        }
    val days =
        costs.mapIndexed {
            index,
            cost,
            ->
            CostDay(day = "2026-11-0${index + 1}", costMicros = cost)
        }
    val total = costs.sum()
    return CostSummary(
        today = CostTotals(tokensIn = 212_000L, tokensOut = 18_000L, costMicros = costs.last()),
        last7Days = CostTotals(tokensIn = 1_200_000L, tokensOut = 90_000L, costMicros = total),
        last30Days =
            CostTotals(tokensIn = 4_800_000L, tokensOut = 320_000L, costMicros = total),
        byDay = days,
        byProject =
            if (spend) {
                listOf(
                    ProjectCost("acme", "Acme API", 1_900_000L),
                    ProjectCost("gone", "", 400_000L),
                )
            } else {
                emptyList()
            },
        byModel =
            if (spend) {
                listOf(
                    ModelCost(ProviderView.ANTHROPIC, "claude-sonnet-5-5", 2_100_000L),
                )
            } else {
                emptyList()
            },
    )
}
