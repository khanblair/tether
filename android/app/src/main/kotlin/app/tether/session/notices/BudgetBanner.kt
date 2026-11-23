package app.tether.session.notices

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.tooling.preview.Preview
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.tokens.TetherSpace
import app.tether.session.agent.AgentBanner
import app.tether.session.agent.BudgetScope

/**
 * The budget warning: the spend passed a share of the [scope] budget. It is information only,
 * so it has no buttons.
 */
@Composable
fun BudgetWarningBanner(
    warning: AgentBanner.BudgetWarning,
    modifier: Modifier = Modifier,
) {
    InlineBanner(
        text =
            stringResource(
                R.string.notices_budget_warning_spent,
                dollarsText(warning.spentMicros),
                dollarsText(warning.limitMicros),
            ),
        tone = BannerTone.Info,
        title = stringResource(budgetWarningRes(warning.scope), warning.percent),
        modifier = modifier.padding(horizontal = TetherSpace.space4),
    )
}

/**
 * The budget stop: the turn is paused until the user continues. [busy] shows the progress state
 * of the Continue call, and [failed] shows the error with Try again. The text button opens the
 * budget settings through [onOpenBudgets].
 */
@Composable
fun BudgetReachedBanner(
    reached: AgentBanner.BudgetReached,
    busy: Boolean,
    failed: Boolean,
    onContinue: () -> Unit,
    onOpenBudgets: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier.padding(horizontal = TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        InlineBanner(
            text =
                stringResource(
                    budgetReachedRes(reached.scope),
                    dollarsText(reached.limitMicros),
                ),
            tone = BannerTone.Warning,
            actionLabel = stringResource(R.string.notices_budget_open),
            onAction = onOpenBudgets,
        )
        if (failed) {
            InlineBanner(
                text = stringResource(R.string.notices_budget_continue_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.notices_try_again),
                onAction = onContinue,
            )
        }
        PrimaryButton(
            label = stringResource(R.string.notices_budget_continue),
            onClick = onContinue,
            enabled = !busy,
            loading = busy,
        )
    }
}

private const val SAMPLE_SPENT_MICROS = 1_640_000L
private const val SAMPLE_SESSION_LIMIT_MICROS = 2_000_000L
private const val SAMPLE_DAY_LIMIT_MICROS = 5_000_000L

@Preview(name = "Budget warning, light")
@Composable
internal fun BudgetWarningPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetWarningBanner(
            AgentBanner.BudgetWarning(
                scope = BudgetScope.Session,
                percent = 82,
                spentMicros = SAMPLE_SPENT_MICROS,
                limitMicros = SAMPLE_SESSION_LIMIT_MICROS,
            ),
        )
    }
}

@Preview(name = "Budget reached, light")
@Composable
internal fun BudgetReachedPreview() {
    TetherTheme(mode = ThemeMode.Light) {
        BudgetReachedBanner(
            reached =
                AgentBanner.BudgetReached(
                    BudgetScope.Session,
                    SAMPLE_SESSION_LIMIT_MICROS,
                    SAMPLE_SESSION_LIMIT_MICROS,
                ),
            busy = false,
            failed = false,
            onContinue = {},
            onOpenBudgets = {},
        )
    }
}

@Preview(name = "Budget reached, dark")
@Composable
internal fun BudgetReachedDarkPreview() {
    TetherTheme(mode = ThemeMode.Dark) {
        BudgetReachedBanner(
            reached =
                AgentBanner.BudgetReached(
                    BudgetScope.Day,
                    SAMPLE_DAY_LIMIT_MICROS,
                    SAMPLE_DAY_LIMIT_MICROS,
                ),
            busy = false,
            failed = true,
            onContinue = {},
            onOpenBudgets = {},
        )
    }
}
