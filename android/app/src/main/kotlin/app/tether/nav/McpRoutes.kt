package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.di.AppModule
import app.tether.settings.mcp.AddMcpServerRoute
import app.tether.settings.mcp.AddMcpServerViewModel
import app.tether.settings.mcp.McpServerDetailScreen
import app.tether.settings.mcp.McpServerDetailViewModel
import app.tether.settings.mcp.McpServersScreen
import app.tether.settings.mcp.McpServersViewModel

private const val MCP_SERVER_ROUTE = "settings/mcp/{serverId}"

private fun mcpServerPath(serverId: String) = "settings/mcp/${Uri.encode(serverId)}"

/** The MCP server list, with its add sheet, and one server's detail screen. */
internal fun NavGraphBuilder.mcpRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_MCP_ROUTE) { McpListRoute(module, nav) }
    composable(
        MCP_SERVER_ROUTE,
        arguments = listOf(navArgument("serverId") { type = NavType.StringType }),
    ) { entry ->
        val serverId = entry.arguments?.getString("serverId").orEmpty()
        McpServerDetailScreen(
            viewModel =
                viewModel(
                    key = serverId,
                    factory = McpServerDetailViewModel.factory(serverId, module.mcpGateway),
                ),
            onBack = nav::back,
            onRemoved = nav::back,
        )
    }
}

/** The list reloads each time it comes back to the front, so a removed server is gone. */
@Composable
private fun McpListRoute(
    module: AppModule,
    nav: TetherNav,
) {
    val list: McpServersViewModel =
        viewModel(factory = McpServersViewModel.factory(module.mcpGateway))
    var adding by remember { mutableStateOf(false) }
    LifecycleResumeEffect(list) {
        list.reload()
        onPauseOrDispose {}
    }
    McpServersScreen(
        viewModel = list,
        onBack = nav::back,
        onOpenServer = { nav.open(mcpServerPath(it)) },
        onAddServer = { adding = true },
    )
    if (adding) {
        AddMcpServerRoute(
            viewModel = viewModel(factory = AddMcpServerViewModel.factory(module.mcpGateway)),
            onDone = {
                adding = false
                list.reload()
            },
        )
    }
}
