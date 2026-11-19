package app.tether.nav

import android.content.pm.ApplicationInfo
import androidx.activity.compose.BackHandler
import androidx.compose.material3.DrawerState
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import app.tether.core.generated.ProviderView
import app.tether.di.AppModule
import app.tether.home.AttentionItem
import app.tether.home.AttentionScreen
import app.tether.home.AttentionViewModel
import app.tether.home.HomeDrawer
import app.tether.home.NewProjectChoice
import app.tether.home.ProjectsScreen
import app.tether.home.ProjectsViewModel
import app.tether.providers.ProviderDetailScreen
import app.tether.providers.ProviderDetailViewModel
import app.tether.providers.ProvidersScreen
import app.tether.providers.ProvidersViewModel
import app.tether.shell.PlaceholderScreen
import app.tether.shell.SettingsScreen
import app.tether.updates.AboutScreen
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/**
 * The navigation graph inside the home drawer. The drawer opens from the Projects menu button and
 * from the left edge, and only on the Projects screen. Shared routes live here; feature modules
 * plug in as they appear.
 */
@Composable
fun TetherNavHost(module: AppModule) {
    val navController = rememberNavController()
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    val scope = rememberCoroutineScope()
    val nav =
        remember(navController, drawerState, scope) {
            TetherNav(navController, drawerState, scope)
        }
    val attentionModel: AttentionViewModel =
        viewModel(factory = AttentionViewModel.factory(module.home.sessions, module.updates.state))
    val attention by attentionModel.items.collectAsStateWithLifecycle()
    val currentRoute =
        navController
            .currentBackStackEntryAsState()
            .value
            ?.destination
            ?.route
    BackHandler(enabled = drawerState.isOpen) { scope.launch { drawerState.close() } }
    ModalNavigationDrawer(
        drawerState = drawerState,
        gesturesEnabled = currentRoute == Route.Projects.path,
        drawerContent = { DrawerHost(module = module, nav = nav, drawerState = drawerState) },
    ) {
        NavHost(navController = navController, startDestination = Route.Projects.path) {
            homeRoutes(module = module, nav = nav, attentionItems = attention)
            settingsRoutes(module = module, nav = nav)
            placeholderRoutes(nav = nav)
        }
    }
}

/** The drawer's content. Its query and session list live here, and every item closes the drawer. */
@Composable
private fun DrawerHost(
    module: AppModule,
    nav: TetherNav,
    drawerState: DrawerState,
) {
    var query by rememberSaveable { mutableStateOf("") }
    val sessions = remember { module.home.sessions.sessions() }
    val now = remember(drawerState.currentValue) { System.currentTimeMillis() }
    HomeDrawer(
        sessions = sessions,
        query = query,
        onQueryChange = { query = it },
        onOpenSession = { nav.closeThen { nav.go(Route.Session) } },
        onAllProjects = { nav.closeThen { nav.toProjects() } },
        onNewSession = { nav.closeThen { nav.go(Route.NewSession) } },
        onOpenSettings = { nav.closeThen { nav.go(Route.Settings) } },
        onOpenSkills = { nav.closeThen { nav.go(Route.Skills) } },
        now = now,
    )
}

/** The navigation actions the screens share. Each one is a plain call on the controller. */
private class TetherNav(
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

    fun toProjects() {
        controller.popBackStack(Route.Projects.path, inclusive = false)
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

private fun NavGraphBuilder.homeRoutes(
    module: AppModule,
    nav: TetherNav,
    attentionItems: List<AttentionItem>,
) {
    composable(Route.Projects.path) {
        val projects: ProjectsViewModel =
            viewModel(factory = ProjectsViewModel.factory(module.home.projects))
        ProjectsScreen(
            viewModel = projects,
            hasAttention = attentionItems.isNotEmpty(),
            attentionCount = attentionItems.size,
            onOpenMenu = nav::openDrawer,
            onOpenAttention = { nav.go(Route.Attention) },
            onOpenProject = { nav.go(Route.ProjectHome) },
            onNewProject = { choice -> nav.go(choice.route()) },
        )
    }
    composable(Route.Attention.path) {
        AttentionScreen(
            items = attentionItems,
            onOpenSession = { nav.go(Route.Session) },
            onOpenAbout = { nav.go(Route.About) },
            onBack = nav::back,
        )
    }
}

private fun NavGraphBuilder.settingsRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.Settings.path) {
        SettingsScreen(
            module = module,
            onBack = nav::back,
            onOpenProviders = { nav.go(Route.Providers) },
            onOpenAbout = { nav.go(Route.About) },
        )
    }
    composable(Route.About.path) {
        AboutScreen(
            controller = module.updates,
            core = module.core,
            showCoreCard = isDebuggable(LocalContext.current.applicationInfo),
            onBack = nav::back,
        )
    }
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

/** The routes that still show the placeholder. Each gets its real screen in a later phase. */
private fun NavGraphBuilder.placeholderRoutes(nav: TetherNav) {
    val real = setOf(Route.Projects, Route.Attention, Route.Settings, Route.Providers, Route.About)
    Route.entries.filter { it !in real }.forEach { route ->
        composable(route.path) { PlaceholderScreen(route = route, onBack = nav::back) }
    }
}

private fun NewProjectChoice.route(): Route =
    when (this) {
        NewProjectChoice.Clone -> Route.CloneProject
        NewProjectChoice.Empty -> Route.NewProject
        NewProjectChoice.Import -> Route.ImportProject
    }

/** True for a debuggable build. The Rust core card shows only in those. */
private fun isDebuggable(info: ApplicationInfo): Boolean =
    info.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0
