package app.tether.session

import app.tether.providers.ProviderGateway

/**
 * What the session sheets act on: one session, its gateways, and the hooks that keep the live
 * connection in step. [reopen] runs after a model or mode change, because the core drops the old
 * runner then. [isRunning] tells the sheets whether a turn is running right now.
 */
data class SessionSheetDeps(
    val sessionId: String,
    val settings: SessionSettingsGateway,
    val providers: ProviderGateway,
    val rewinder: SessionRewinder,
    val isRunning: () -> Boolean,
    val reopen: () -> Unit,
)
