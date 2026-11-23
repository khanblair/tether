package app.tether.nav

import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.settings.budgets.BudgetsScreen
import app.tether.settings.budgets.BudgetsViewModel
import app.tether.settings.budgets.CostScreen
import app.tether.settings.budgets.CostViewModel

/** The budget limits in Settings and the Cost screen opened from the drawer. */
internal fun NavGraphBuilder.budgetsRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_BUDGETS_ROUTE) {
        val budgets: BudgetsViewModel =
            viewModel(
                factory =
                    viewModelFactory {
                        initializer {
                            BudgetsViewModel(
                                module.budgetsGateway,
                            )
                        }
                    },
            )
        val state by budgets.state.collectAsStateWithLifecycle()
        BudgetsScreen(
            state = state,
            onDailyChange = budgets::onDailyChange,
            onSessionChange = budgets::onSessionChange,
            onRemoveDaily = budgets::removeDaily,
            onRemoveSession = budgets::removeSession,
            onSave = budgets::save,
            onRetry = budgets::retry,
            onBack = nav::back,
        )
    }
    composable(Route.Cost.path) {
        val cost: CostViewModel =
            viewModel(
                factory = viewModelFactory { initializer { CostViewModel(module.budgetsGateway) } },
            )
        val state by cost.state.collectAsStateWithLifecycle()
        CostScreen(
            state = state,
            onSelect = cost::select,
            onRetry = cost::retry,
            onBack = nav::back,
        )
    }
}
