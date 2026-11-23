package app.tether.settings.budgets

/**
 * A budgets gateway for tests. It keeps the limits in memory, records every save and summary
 * request, and throws [loadFailure], [saveFailure] or [summaryFailure] when one is set.
 */
class FakeBudgetsGateway(
    var limits: BudgetLimits = BudgetLimits(dailyMicros = null, sessionMicros = null),
    var summary: CostSummary = summaryWith(),
) : BudgetsGateway {
    var loadFailure: BudgetsFailure? = null
    var saveFailure: BudgetsFailure? = null
    var summaryFailure: BudgetsFailure? = null

    /** Every call to [setBudgets], in order, including the ones that failed. */
    val saves = mutableListOf<Pair<Long?, Long?>>()

    /** Every call to [usageSummary] as (days, offset), in order. */
    val summaryCalls = mutableListOf<Pair<Int, Int>>()

    override fun budgets(): BudgetLimits {
        loadFailure?.let { throw it }
        return limits
    }

    override fun setBudgets(
        dailyMicros: Long?,
        sessionMicros: Long?,
    ) {
        saves += dailyMicros to sessionMicros
        saveFailure?.let { throw it }
        limits = BudgetLimits(dailyMicros, sessionMicros)
    }

    override fun usageSummary(
        days: Int,
        utcOffsetMinutes: Int,
    ): CostSummary {
        summaryCalls += days to utcOffsetMinutes
        summaryFailure?.let { throw it }
        return summary
    }
}

/** Totals with only a cost. */
internal fun totalsOf(costMicros: Long): CostTotals =
    CostTotals(tokensIn = 0L, tokensOut = 0L, costMicros = costMicros)

/** A summary whose days are [days], with no projects or models. */
internal fun summaryWith(days: List<CostDay> = emptyList()): CostSummary =
    CostSummary(
        today = totalsOf(0L),
        last7Days = totalsOf(0L),
        last30Days = totalsOf(0L),
        byDay = days,
        byProject = emptyList(),
        byModel = emptyList(),
    )

/** One day per cost, starting on 1 November 2026. */
internal fun daysOf(vararg costs: Long): List<CostDay> =
    costs.mapIndexed {
        index,
        cost,
        ->
        CostDay(day = "2026-11-%02d".format(index + 1), costMicros = cost)
    }
