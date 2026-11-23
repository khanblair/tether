package app.tether.settings.budgets

import app.tether.core.generated.UsageRuntime
import app.tether.core.generated.UsageRuntimeException

/**
 * What the budget and cost screens need from the Rust core. Every call blocks, so callers run it
 * off the main thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface BudgetsGateway {
    /** The saved daily and default session limits. */
    fun budgets(): BudgetLimits

    /** Saves both limits in one call. Null in either slot means no limit. */
    fun setBudgets(
        dailyMicros: Long?,
        sessionMicros: Long?,
    )

    /** Spend for the last [days] days (1 to 90), shifted by the device's [utcOffsetMinutes]. */
    fun usageSummary(
        days: Int,
        utcOffsetMinutes: Int,
    ): CostSummary
}

/**
 * The real gateway over the UniFFI [UsageRuntime]. Each core error becomes a [BudgetsFailure]
 * with the matching [BudgetsProblem].
 */
class CoreBudgetsGateway(
    private val runtime: Lazy<UsageRuntime>,
) : BudgetsGateway {
    private val usage: UsageRuntime by runtime

    override fun budgets(): BudgetLimits =
        guard {
            usage.budgets().let {
                BudgetLimits(
                    dailyMicros = it.dailyLimitMicros?.toLong(),
                    sessionMicros = it.sessionLimitMicros?.toLong(),
                )
            }
        }

    override fun setBudgets(
        dailyMicros: Long?,
        sessionMicros: Long?,
    ) = guard {
        usage.setBudgets(dailyMicros?.toULong(), sessionMicros?.toULong())
    }

    override fun usageSummary(
        days: Int,
        utcOffsetMinutes: Int,
    ): CostSummary = guard { usage.usageSummary(days.toUInt(), utcOffsetMinutes).toCostSummary() }

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: UsageRuntimeException) {
            throw BudgetsFailure(failure.problem(), failure)
        }
}

/** The problem a core error stands for. */
internal fun UsageRuntimeException.problem(): BudgetsProblem =
    when (this) {
        is UsageRuntimeException.Unavailable -> BudgetsProblem.Unavailable
        is UsageRuntimeException.NotFound -> BudgetsProblem.NotFound
        is UsageRuntimeException.InvalidBudget -> BudgetsProblem.InvalidBudget
    }
