package app.tether.nav

import androidx.compose.runtime.Composable
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import app.tether.di.AppModule
import app.tether.shell.PlaceholderScreen
import app.tether.shell.ProjectsScreen
import app.tether.shell.SettingsScreen

/** The navigation graph. Shared routes live here; feature modules plug in as they appear. */
@Composable
fun TetherNavHost(module: AppModule) {
    val navController = rememberNavController()
    val back: () -> Unit = { navController.popBackStack() }

    NavHost(navController = navController, startDestination = Route.Projects.path) {
        composable(Route.Projects.path) {
            ProjectsScreen(
                core = module.core,
                onOpen = { route -> navController.navigate(route.path) },
            )
        }
        composable(Route.Settings.path) {
            SettingsScreen(module = module, onBack = back)
        }
        Route.entries
            .filter { it != Route.Projects && it != Route.Settings }
            .forEach { route ->
                composable(route.path) { PlaceholderScreen(route = route, onBack = back) }
            }
    }
}
