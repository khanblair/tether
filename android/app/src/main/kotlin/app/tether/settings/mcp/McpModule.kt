package app.tether.settings.mcp

import app.tether.core.generated.McpRuntime
import app.tether.core.generated.SecretVault

/**
 * The MCP gateway over the app's database and secret vault. The runtime opens on the first call,
 * off the main thread, so creating the gateway never blocks the UI. Not wired into Koin yet.
 */
fun createMcpGateway(
    databasePath: String,
    vault: SecretVault,
): McpGateway = CoreMcpGateway(lazy { McpRuntime(databasePath, vault) })
