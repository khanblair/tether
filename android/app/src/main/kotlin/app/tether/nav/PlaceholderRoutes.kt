package app.tether.nav

import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.shell.PlaceholderScreen

/** The routes that still show the placeholder. Each gets its real screen in a later phase. */
internal fun NavGraphBuilder.placeholderRoutes(nav: TetherNav) {
    val real =
        setOf(
            Route.Home,
            Route.Projects,
            Route.ProjectHome,
            Route.Attention,
            Route.Settings,
            Route.Providers,
            Route.About,
        )
    Route.entries.filter { it !in real && it != Route.Session }.forEach { route ->
        composable(route.path) { PlaceholderScreen(route = route, onBack = nav::back) }
    }
}
