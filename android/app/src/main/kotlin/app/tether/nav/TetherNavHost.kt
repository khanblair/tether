package app.tether.nav

import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.DrawerState
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import app.tether.core.generated.EventSink
import app.tether.core.generated.SessionRuntime
import app.tether.design.components.TetherSnackbarHost
import app.tether.di.AppModule
import app.tether.home.AttentionViewModel
import app.tether.home.HomeDrawer
import app.tether.session.TranscriptScreen
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The app's navigation graph and its chrome: the drawer that opens from the top bars, and the
 * snackbar host that shows messages from [AppModule.messages]. Screens live in the route files.
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
    val snackbars = remember { SnackbarHostState() }
    BackHandler(enabled = drawerState.isOpen) { scope.launch { drawerState.close() } }
    LaunchedEffect(module) {
        module.messages.flow.collect { message ->
            val result =
                snackbars.showSnackbar(
                    message = message.text,
                    actionLabel = message.actionLabel,
                    duration =
                        if (message.actionLabel == null) {
                            SnackbarDuration.Short
                        } else {
                            SnackbarDuration.Long
                        },
                )
            if (result == SnackbarResult.ActionPerformed) message.onAction?.invoke()
        }
    }
    ModalNavigationDrawer(
        drawerState = drawerState,
        gesturesEnabled = currentRoute == Route.Home.path || currentRoute == Route.Projects.path,
        drawerContent = {
            DrawerHost(
                module = module,
                nav = nav,
                drawerState = drawerState,
                currentRoute = currentRoute,
                attentionCount = attention.size,
            )
        },
    ) {
        Box(modifier = Modifier.fillMaxSize()) {
            NavHost(navController = navController, startDestination = Route.Home.path) {
                homeRoutes(module = module, nav = nav, attentionItems = attention)
                settingsRoutes(module = module, nav = nav)
                placeholderRoutes(nav = nav)
                composable("session/{sessionId}") { entry ->
                    TranscriptScreen(
                        sessionId = entry.arguments?.getString("sessionId"),
                        load = { id ->
                            withContext(Dispatchers.IO) {
                                val sink =
                                    object : EventSink {
                                    override fun onEvent(
                                        event: app.tether.core.generated.CoreEventView,
                                    ) = Unit
                                }
                                SessionRuntime(module.databasePath, module.vault, sink)
                                    .use { it.transcript(id) }
                            }
                        },
                        onBack = nav::back,
                    )
                }
            }
            TetherSnackbarHost(
                hostState = snackbars,
                modifier =
                    Modifier
                        .align(Alignment.BottomCenter)
                        .windowInsetsPadding(WindowInsets.navigationBars),
            )
        }
    }
}

/** The drawer's content. Its query and session list live here, and every item closes the drawer. */
@Composable
private fun DrawerHost(
    module: AppModule,
    nav: TetherNav,
    drawerState: DrawerState,
    currentRoute: String?,
    attentionCount: Int,
) {
    var query by rememberSaveable { mutableStateOf("") }
    val sessions = remember { module.home.sessions.sessions() }
    val themeMode by module.themeMode.collectAsStateWithLifecycle()
    val now = remember(drawerState.currentValue) { System.currentTimeMillis() }
    HomeDrawer(
        sessions = sessions,
        query = query,
        onQueryChange = { query = it },
        currentRoute = currentRoute,
        attentionCount = attentionCount,
        themeMode = themeMode,
        onToggleTheme = { module.theme.cycle() },
        onDestination = { route -> nav.closeThen { nav.goTopLevel(route) } },
        onOpenSession = { nav.closeThen { nav.open("session/${Uri.encode(it)}") } },
        onNewSession = { nav.closeThen { nav.go(Route.NewSession) } },
        onOpenSettings = { nav.closeThen { nav.go(Route.Settings) } },
        onOpenSkills = { nav.closeThen { nav.go(Route.Skills) } },
        now = now,
    )
}
