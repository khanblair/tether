package app.tether.nav

import android.content.pm.ApplicationInfo
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.core.generated.ProviderView
import app.tether.di.AppModule
import app.tether.providers.ProviderDetailScreen
import app.tether.providers.ProviderDetailViewModel
import app.tether.providers.ProvidersScreen
import app.tether.providers.ProvidersViewModel
import app.tether.shell.SettingsScreen
import app.tether.updates.AboutScreen

/** Settings, About, the provider list, and a provider's detail screen. */
internal fun NavGraphBuilder.settingsRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.Settings.path) {
        SettingsScreen(
            module = module,
            settings = settingsDepsOf(LocalContext.current),
            onBack = nav::back,
            onOpen = nav::open,
            onOpenProviders = { nav.go(Route.Providers) },
            onOpenAbout = { nav.go(Route.About) },
        )
    }
    aboutRoute(module, nav)
    settingsAreaRoutes(module = module, nav = nav)
    githubRoutes(module = module, nav = nav)
    mcpRoutes(module = module, nav = nav)
    executorsRoutes(module = module, nav = nav)
    composable(Route.Providers.path) {
        ProvidersScreen(
            viewModel = viewModel(factory = ProvidersViewModel.factory(module.providerGateway)),
            onOpen = { provider -> nav.open(providerDetailPath(provider)) },
            onBack = nav::back,
        )
    }
    composable(
        Routes.PROVIDER_DETAIL,
        arguments = listOf(navArgument("provider") { type = NavType.StringType }),
    ) { entry ->
        val raw = entry.arguments?.getString("provider")
        val provider = ProviderView.entries.firstOrNull { it.name == raw }
        if (provider == null) {
            LaunchedEffect(Unit) { nav.back() }
        } else {
            ProviderDetailScreen(
                viewModel =
                    viewModel(
                        key = provider.name,
                        factory = ProviderDetailViewModel.factory(provider, module.providerGateway),
                    ),
                onBack = nav::back,
            )
        }
    }
}

/** The About screen. Its replay row clears the first-run flag and opens the flow. */
private fun NavGraphBuilder.aboutRoute(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.About.path) {
        val settings = settingsDepsOf(LocalContext.current).store
        AboutScreen(
            controller = module.updates,
            core = module.core,
            messages = module.messages,
            showCoreCard = isDebuggable(LocalContext.current.applicationInfo),
            onShowWelcome = {
                settings.saveOnboarded(false)
                nav.go(Route.Onboarding)
            },
            onBack = nav::back,
        )
    }
}

/** True for a debuggable build. The Rust core card shows only in those. */
private fun isDebuggable(info: ApplicationInfo): Boolean =
    info.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
