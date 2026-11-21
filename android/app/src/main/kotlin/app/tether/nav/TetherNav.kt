package app.tether.nav

import androidx.compose.material3.DrawerState
import androidx.navigation.NavHostController
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/** The navigation actions the screens share. Each one is a plain call on the controller. */
internal class TetherNav(
    private val controller: NavHostController,
    private val drawer: DrawerState,
    private val scope: CoroutineScope,
) {
    fun go(route: Route) = open(route.path)

    fun open(path: String) {
        controller.navigate(path)
    }

    fun back() {
        controller.popBackStack()
    }

    /**
     * Opens a top-level destination from the drawer. Screens above Home are cleared, so Back from
     * the destination goes to Home. Home itself pops back to its own entry.
     */
    fun goTopLevel(route: Route) {
        if (route == Route.Home) {
            controller.popBackStack(Route.Home.path, inclusive = false)
            return
        }
        controller.navigate(route.path) {
            popUpTo(Route.Home.path) { inclusive = false }
            launchSingleTop = true
        }
    }

    /** Opens [path] and removes [removing] from the back stack, so Back skips it. */
    fun replace(
        path: String,
        removing: Route,
    ) {
        controller.navigate(path) {
            popUpTo(removing.path) { inclusive = true }
        }
    }

    fun openDrawer() {
        scope.launch { drawer.open() }
    }

    /** Closes the drawer first, then runs [action]. */
    fun closeThen(action: () -> Unit) {
        scope.launch {
            drawer.close()
            action()
        }
    }
}
