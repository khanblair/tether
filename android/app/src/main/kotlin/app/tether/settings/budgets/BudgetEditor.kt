package app.tether.settings.budgets

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace

/**
 * The two limit fields, the explanation of what a limit does, and the Save button. Each field
 * has a Remove limit action that empties it. A blank field saves as no limit. The problems are
 * shown under each field as the person types.
 */
@Composable
fun BudgetEditor(
    daily: String,
    session: String,
    dailyProblem: DollarProblem?,
    sessionProblem: DollarProblem?,
    canSave: Boolean,
    saving: Boolean,
    onDailyChange: (String) -> Unit,
    onSessionChange: (String) -> Unit,
    onRemoveDaily: () -> Unit,
    onRemoveSession: () -> Unit,
    onSave: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        LimitField(
            label = stringResource(R.string.budgets_daily_label),
            value = daily,
            problem = dailyProblem,
            onChange = onDailyChange,
            onRemove = onRemoveDaily,
        )
        LimitField(
            label = stringResource(R.string.budgets_session_label),
            value = session,
            problem = sessionProblem,
            onChange = onSessionChange,
            onRemove = onRemoveSession,
        )
        Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
            Text(
                text = stringResource(R.string.budgets_explain),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            Text(
                text = stringResource(R.string.budgets_warning),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        PrimaryButton(
            label = stringResource(R.string.budgets_save),
            onClick = onSave,
            enabled = canSave,
            loading = saving,
        )
    }
}

/** One dollar field with its supporting line and a Remove limit action under it. */
@Composable
private fun LimitField(
    label: String,
    value: String,
    problem: DollarProblem?,
    onChange: (String) -> Unit,
    onRemove: () -> Unit,
) {
    val support =
        problem?.let { stringResource(it.messageRes()) }
            ?: stringResource(R.string.budgets_limit_hint)
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        OutlinedTextField(
            value = value,
            onValueChange = onChange,
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            label = { Text(label) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            isError = problem != null,
            supportingText = { Text(support) },
        )
        TetherTextButton(
            label = stringResource(R.string.budgets_remove_limit),
            onClick = onRemove,
            enabled = value.isNotBlank(),
        )
    }
}
