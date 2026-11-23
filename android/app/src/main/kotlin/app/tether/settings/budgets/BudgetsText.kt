package app.tether.settings.budgets

import androidx.annotation.StringRes
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R

/** The message for a limit that is not valid, in plain words. */
@StringRes
internal fun DollarProblem.messageRes(): Int =
    when (this) {
        DollarProblem.NotANumber -> R.string.budgets_problem_not_number
        DollarProblem.TooManyDecimals -> R.string.budgets_problem_decimals
        DollarProblem.Negative -> R.string.budgets_problem_negative
        DollarProblem.BelowMinimum -> R.string.budgets_problem_below
        DollarProblem.AboveMaximum -> R.string.budgets_problem_above
    }

/** The message for a failed save. */
@StringRes
internal fun BudgetsProblem.saveMessageRes(): Int =
    when (this) {
        BudgetsProblem.InvalidBudget -> R.string.budgets_invalid_range
        BudgetsProblem.Unavailable, BudgetsProblem.NotFound -> R.string.budgets_save_failed
    }

/** The amount in dollars for people to read, with the words for a spend under one cent. */
@Composable
internal fun dollarsText(micros: Long): String =
    formatDollars(micros, stringResource(R.string.budgets_less_than_a_cent))
