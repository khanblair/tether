package app.tether.nav

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import app.tether.core.generated.ProviderView
import app.tether.di.AppModule
import app.tether.providers.ProviderDetailScreen
import app.tether.providers.ProviderDetailViewModel
import app.tether.providers.ProvidersScreen
import app.tether.providers.ProvidersViewModel
import app.tether.shell.PlaceholderScreen
import app.tether.shell.ProjectsScreen
import app.tether.shell.SettingsScreen
import app.tether.updates.AboutScreen

/** The navigation graph. Shared routes live here; feature modules plug in as they appear. */
@Composable
fun TetherNavHost(module: AppModule) {
    val navController = rememberNavController()
    val back: () -> Unit = { navController.popBackStack() }
    val openAbout: () -> Unit = { navController.navigate(Route.About.path) }

    NavHost(navController = navController, startDestination = Route.Projects.path) {
        composable(Route.Projects.path) {
            ProjectsScreen(
                core = module.core,
                updates = module.updates.state,
                onOpenAbout = openAbout,
                onOpen = { route -> navController.navigate(route.path) },
            )
        }
        composable(Route.Settings.path) {
            SettingsScreen(
                module = module,
                onBack = back,
                onOpenProviders = { navController.navigate(Route.Providers.path) },
                onOpenAbout = openAbout,
            )
        }
        composable(Route.About.path) {
            AboutScreen(controller = module.updates, onBack = back)
        }
        composable(Route.Providers.path) {
            ProvidersScreen(
                viewModel = viewModel(factory = ProvidersViewModel.factory(module.providerGateway)),
                onOpen = { provider -> navController.navigate(providerDetailPath(provider)) },
                onBack = back,
            )
        }
        composable(
            Routes.PROVIDER_DETAIL,
            arguments = listOf(navArgument("provider") { type = NavType.StringType }),
        ) { entry ->
            val raw = entry.arguments?.getString("provider")
            val provider = ProviderView.entries.firstOrNull { it.name == raw }
            if (provider == null) {
                LaunchedEffect(Unit) { back() }
            } else {
                ProviderDetailScreen(
                    viewModel =
                        viewModel(
                            key = provider.name,
                            factory =
                                ProviderDetailViewModel.factory(
                                    provider,
                                    module.providerGateway,
                                ),
                        ),
                    onBack = back,
                )
            }
        }
        Route.entries
            .filter {
                it != Route.Projects &&
                    it != Route.Settings &&
                    it != Route.Providers &&
                    it != Route.About
            }.forEach { route ->
                composable(route.path) { PlaceholderScreen(route = route, onBack = back) }
            }
    }
}
