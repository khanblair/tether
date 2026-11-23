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
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.settings.SettingsPage

/**
 * The Budgets settings page: the form for the daily and default session limits. Every state
 * (loading, load failure, editing, save failure, saved) comes from [state].
 */
@Composable
fun BudgetsScreen(
    state: BudgetsUiState,
    onDailyChange: (String) -> Unit,
    onSessionChange: (String) -> Unit,
    onRemoveDaily: () -> Unit,
    onRemoveSession: () -> Unit,
    onSave: () -> Unit,
    onRetry: () -> Unit,
    onBack: () -> Unit,
) {
    SettingsPage(title = stringResource(R.string.budgets_title), onBack = onBack) {
        Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space4)) {
            Text(
                text = stringResource(R.string.budgets_intro),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            when {
                state.loadFailed -> LoadFailure(onRetry)
                state.loading -> CircularProgressIndicator(color = TetherTheme.colors.text3)
                else ->
                    BudgetEditor(
                        daily = state.daily,
                        session = state.session,
                        dailyProblem = (state.dailyInput as? DollarInput.Invalid)?.problem,
                        sessionProblem = (state.sessionInput as? DollarInput.Invalid)?.problem,
                        canSave = state.canSave,
                        saving = state.saving,
                        onDailyChange = onDailyChange,
                        onSessionChange = onSessionChange,
                        onRemoveDaily = onRemoveDaily,
                        onRemoveSession = onRemoveSession,
                        onSave = onSave,
                    )
            }
            if (state.saved) {
                InlineBanner(
                    text = stringResource(R.string.budgets_saved),
                    tone = BannerTone.Success,
                )
            }
            state.saveFailure?.let { problem ->
                InlineBanner(
                    text = stringResource(problem.saveMessageRes()),
                    tone = BannerTone.Danger,
                )
            }
        }
    }
}

/** The message and Try again for a failed load of the saved limits. */
@Composable
private fun LoadFailure(onRetry: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        InlineBanner(
            text = stringResource(R.string.budgets_load_failed),
            tone = BannerTone.Danger,
        )
        TetherTextButton(
            label = stringResource(R.string.budgets_try_again),
            onClick = onRetry,
        )
    }
}
