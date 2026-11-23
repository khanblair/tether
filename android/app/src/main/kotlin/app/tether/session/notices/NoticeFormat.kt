package app.tether.session.notices

import androidx.annotation.StringRes
import app.tether.R
import app.tether.session.agent.BudgetScope
import java.util.Locale

/** How full the context is. The band sets the tone of the context meter. */
enum class ContextBand {
    /** Under 60 percent. */
    Neutral,

    /** From 60 to 80 percent, both included. */
    Warning,

    /** Over 80 percent. */
    Danger,
}

/** Where a session's spend stands against its budget. */
enum class BudgetState {
    /** No budget is set, or it is zero. */
    NoBudget,

    /** Under 80 percent of the budget. */
    Under,

    /** From 80 percent of the budget up to the limit. */
    Near,

    /** The spend reached the limit. */
    Reached,
}

/** A spend amount as the meters show it: a dollar text, or a note that it is under one cent. */
sealed interface CostAmount {
    /** A whole amount in dollars, such as "$0.42". */
    data class Dollars(
        val text: String,
    ) : CostAmount

    /** Some spend, but less than one cent. */
    data object UnderCent : CostAmount
}

private const val WARNING_FROM_PERCENT = 60
private const val DANGER_ABOVE_PERCENT = 80
private const val NEAR_BUDGET_PERCENT = 80L
private const val PERCENT = 100L
private const val MICROS_PER_CENT = 10_000L
private const val MICROS_PER_DOLLAR = 1_000_000.0

/** The band for a context use of [percent]. The edges belong to the lower band. */
fun contextBand(percent: Int): ContextBand =
    when {
        percent > DANGER_ABOVE_PERCENT -> ContextBand.Danger
        percent >= WARNING_FROM_PERCENT -> ContextBand.Warning
        else -> ContextBand.Neutral
    }

/** A dollar amount with two decimals, such as "$2.00". Zero shows as "$0.00". */
fun dollarsText(micros: Long): String =
    "\$" + String.format(Locale.US, "%.2f", micros / MICROS_PER_DOLLAR)

/** The spend as the meters show it. Above zero but under one cent is [CostAmount.UnderCent]. */
fun costAmount(micros: Long): CostAmount =
    when {
        micros <= 0L -> CostAmount.Dollars(dollarsText(0L))
        micros < MICROS_PER_CENT -> CostAmount.UnderCent
        else -> CostAmount.Dollars(dollarsText(micros))
    }

/** Where [spentMicros] stands against [budgetMicros]. A null or zero budget means no budget. */
fun budgetState(
    spentMicros: Long,
    budgetMicros: Long?,
): BudgetState =
    when {
        budgetMicros == null || budgetMicros <= 0L -> BudgetState.NoBudget
        spentMicros >= budgetMicros -> BudgetState.Reached
        spentMicros * PERCENT >= budgetMicros * NEAR_BUDGET_PERCENT -> BudgetState.Near
        else -> BudgetState.Under
    }

/** The warning title for a [scope]. It takes the percent as its only argument. */
@StringRes
fun budgetWarningRes(scope: BudgetScope): Int =
    when (scope) {
        BudgetScope.Session -> R.string.notices_budget_warning_session
        BudgetScope.Day -> R.string.notices_budget_warning_day
    }

/** The budget stop sentence for a [scope]. It takes the limit as its only argument. */
@StringRes
fun budgetReachedRes(scope: BudgetScope): Int =
    when (scope) {
        BudgetScope.Session -> R.string.notices_budget_reached_session
        BudgetScope.Day -> R.string.notices_budget_reached_day
    }
