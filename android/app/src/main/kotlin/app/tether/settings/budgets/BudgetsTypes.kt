package app.tether.settings.budgets

import androidx.compose.runtime.Immutable
import app.tether.core.generated.ProviderView

/** The two saved limits in USD micros. Null means no limit. */
@Immutable
data class BudgetLimits(
    val dailyMicros: Long?,
    val sessionMicros: Long?,
)

/** Spend and tokens on one UTC day. [day] is `YYYY-MM-DD`. A free model has tokens and no spend. */
@Immutable
data class CostDay(
    val day: String,
    val costMicros: Long,
    val tokensIn: Long = 0L,
    val tokensOut: Long = 0L,
) {
    /** The tokens of the day, in and out. */
    val tokens: Long get() = tokensIn + tokensOut
}

/** Tokens and spend over one window. */
@Immutable
data class CostTotals(
    val tokensIn: Long,
    val tokensOut: Long,
    val costMicros: Long,
)

/** Spend and tokens of one project. [name] is empty when the project was deleted. */
@Immutable
data class ProjectCost(
    val projectId: String,
    val name: String,
    val costMicros: Long,
    val tokensIn: Long = 0L,
    val tokensOut: Long = 0L,
)

/** Spend and tokens of one model. A free model shows its tokens with no spend. */
@Immutable
data class ModelCost(
    val provider: ProviderView,
    val modelId: String,
    val costMicros: Long,
    val tokensIn: Long = 0L,
    val tokensOut: Long = 0L,
)

/** The usage the cost screen shows. [byDay] is zero-filled and oldest first. */
@Immutable
data class CostSummary(
    val today: CostTotals,
    val last7Days: CostTotals,
    val last30Days: CostTotals,
    val byDay: List<CostDay>,
    val byProject: List<ProjectCost>,
    val byModel: List<ModelCost>,
)

/** Why a budgets call failed, in the terms the screens need. */
enum class BudgetsProblem {
    /** The database or usage rows cannot be read or written. */
    Unavailable,

    /** The requested session does not exist. */
    NotFound,

    /** A limit is below one cent or above ten thousand dollars. */
    InvalidBudget,
}

/** A budgets call failed. [problem] says which message to show. */
class BudgetsFailure(
    val problem: BudgetsProblem,
    cause: Throwable? = null,
) : Exception(cause)

/** The problem behind a failed budgets call. Any other failure counts as unavailable. */
internal fun Throwable.budgetsProblem(): BudgetsProblem =
    (this as? BudgetsFailure)?.problem ?: BudgetsProblem.Unavailable

/** What the daily bars measure: spend, or tokens when the usage is all free. */
internal enum class CostMetric { Spend, Tokens }
