package app.tether.settings.budgets

import app.tether.core.generated.UsageRuntime

/**
 * The budgets gateway over the app's database. The runtime opens on the first call, off the main
 * thread, so creating the gateway never blocks the UI.
 */
fun createBudgetsGateway(databasePath: String): BudgetsGateway =
    CoreBudgetsGateway(runtime = lazy { UsageRuntime(databasePath) })
