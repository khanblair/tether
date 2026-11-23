package app.tether.settings.budgets

import app.tether.core.generated.UsageSummaryView
import app.tether.core.generated.UsageTotalsView

/** The core's usage summary as the cost screens hold it. */
internal fun UsageSummaryView.toCostSummary(): CostSummary =
    CostSummary(
        today = today.toTotals(),
        last7Days = last7Days.toTotals(),
        last30Days = last30Days.toTotals(),
        byDay = byDay.map { CostDay(day = it.day, costMicros = it.costMicros.toLong()) },
        byProject =
            byProject.map {
                ProjectCost(
                    projectId = it.projectId,
                    name = it.projectName,
                    costMicros = it.costMicros.toLong(),
                )
            },
        byModel =
            byModel.map {
                ModelCost(
                    provider = it.provider,
                    modelId = it.modelId,
                    costMicros = it.costMicros.toLong(),
                )
            },
    )

private fun UsageTotalsView.toTotals(): CostTotals =
    CostTotals(
        tokensIn = tokensIn.toLong(),
        tokensOut = tokensOut.toLong(),
        costMicros = costMicros.toLong(),
    )
