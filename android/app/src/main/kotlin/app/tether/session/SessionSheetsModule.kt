package app.tether.session

import app.tether.core.generated.SecretVault
import app.tether.core.generated.SessionRuntime

/**
 * The settings gateway for screens that are not inside one session, such as the search screen.
 * The runtime opens on the first call, which runs off the main thread. A session screen uses the
 * gateway of its own connection, so the core can drop its runner after a change.
 */
fun createSessionSettingsGateway(
    databasePath: String,
    vault: SecretVault,
): SessionSettingsGateway =
    CoreSessionSettingsGateway(lazy { SessionRuntime(databasePath, vault, QuietEvents) })
