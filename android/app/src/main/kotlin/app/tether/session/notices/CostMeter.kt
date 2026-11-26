package app.tether.session.notices

import androidx.compose.foundation.clickable
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import app.tether.R
import app.tether.design.TetherTheme

/**
 * The session cost in the status line. [costMicros] is null when the model has no known price,
 * and the meter then says "No price". With a [budgetMicros] limit the text reads "$0.42 of $2.00",
 * and near or reached budgets change the wording and the tone. A tap calls [onClick] when it is
 * set. [inDanger] draws the text in the danger tone for the Bypass line.
 */
@Composable
fun CostMeter(
    costMicros: Long?,
    modifier: Modifier = Modifier,
    budgetMicros: Long? = null,
    onClick: (() -> Unit)? = null,
    inDanger: Boolean = false,
) {
    val tapLabel = stringResource(R.string.notices_cost_open)
    val tap =
        if (onClick == null) {
            Modifier
        } else {
            Modifier
                .minimumInteractiveComponentSize()
                .clickable(role = Role.Button, onClickLabel = tapLabel, onClick = onClick)
        }
    if (costMicros == null) {
        Text(
            text = stringResource(R.string.notices_cost_no_price),
            style = MaterialTheme.typography.labelSmall,
            color = TetherTheme.colors.text3,
            modifier = modifier.then(tap),
        )
        return
    }
    val state = budgetState(costMicros, budgetMicros)
    val spent = spentText(costMicros)
    val text =
        if (budgetMicros == null || state == BudgetState.NoBudget) {
            spent
        } else {
            budgetText(state, spent, dollarsText(budgetMicros))
        }
    Text(
        text = text,
        style = MaterialTheme.typography.labelSmall,
        color = if (inDanger) TetherTheme.colors.danger else stateColor(state),
        fontWeight = if (inDanger) FontWeight.Medium else null,
        modifier = modifier.then(tap),
    )
}

@Composable
private fun spentText(costMicros: Long): String =
    when (val amount = costAmount(costMicros)) {
        is CostAmount.Dollars -> amount.text
        CostAmount.UnderCent -> stringResource(R.string.notices_cost_under_cent)
    }

@Composable
private fun budgetText(
    state: BudgetState,
    spent: String,
    limit: String,
): String =
    when (state) {
        BudgetState.Near -> stringResource(R.string.notices_cost_near, spent, limit)
        BudgetState.Reached -> stringResource(R.string.notices_cost_reached, spent, limit)
        BudgetState.Under,
        BudgetState.NoBudget,
        -> stringResource(R.string.notices_cost_budget, spent, limit)
    }

@Composable
private fun stateColor(state: BudgetState): Color =
    when (state) {
        BudgetState.Reached -> TetherTheme.colors.danger
        BudgetState.Near -> TetherTheme.colors.warning
        BudgetState.Under,
        BudgetState.NoBudget,
        -> TetherTheme.colors.text3
    }
