package app.tether.nav

import android.net.Uri
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.exclude
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.ime
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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import app.tether.R
import app.tether.design.components.TetherSnackbarHost
import app.tether.di.AppModule
import app.tether.home.AttentionViewModel
import app.tether.home.HomeDrawer
import app.tether.home.ProjectSummary
import app.tether.home.SessionRenameHost
import app.tether.home.SessionSummary
import app.tether.home.sessionRenameViewModel
import app.tether.session.SessionSettingsException
import app.tether.share.ShareProjectScreen
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
    val context = LocalContext.current
    val start = remember { startRoute(settingsDepsOf(context).store) }
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
    LaunchedEffect(currentRoute) { attentionModel.refresh() }
    val snackbars = remember { SnackbarHostState() }
    val shared by module.shareInbox.pending.collectAsStateWithLifecycle()
    LaunchedEffect(shared?.value) {
        if (shared != null && currentRoute != "share-project") nav.open("share-project")
    }
    BackHandler(enabled = drawerState.isOpen) { scope.launch { drawerState.close() } }
    SnackbarMessages(module = module, snackbars = snackbars)
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
        // Keyboard space above the navigation bar. The screens' Scaffolds pad the bar already.
        Box(
            modifier =
                Modifier
                    .fillMaxSize()
                    .windowInsetsPadding(WindowInsets.ime.exclude(WindowInsets.navigationBars)),
        ) {
            NavHost(navController = navController, startDestination = start) {
                onboardingRoutes(module = module, nav = nav)
                homeRoutes(module = module, nav = nav, attentionItems = attention)
                settingsRoutes(module = module, nav = nav)
                projectArchiveRoutes(module = module, nav = nav)
                gitRoutes(module = module, nav = nav)
                sessionSheetRoutes(
                    settings = module.sessionSettings,
                    onBack = nav::back,
                    onOpenSession = { nav.open("session/${Uri.encode(it)}") },
                )
                placeholderRoutes(nav = nav)
                memoryRoutes(gateways = module.memoryArea, nav = nav)
                filesRoutes(gateway = module.filesGateway, messages = module.messages, nav = nav)
                previewRoutes(module = module, nav = nav)
                linuxRoutes(module = module, nav = nav)
                runsRoutes(module = module, nav = nav)
                newSessionRoutes(module = module, nav = nav)
                composable("share-project") {
                    val projects by produceState(emptyList<ProjectSummary>()) {
                        value = withContext(Dispatchers.IO) { module.home.projects.projects() }
                    }
                    ShareProjectScreen(
                        shared = shared,
                        projects = projects,
                        onChoose = module.shareInbox::chooseProject,
                        onBack = {
                            module.shareInbox.clear()
                            nav.back()
                        },
                    )
                }
                composable("session/{sessionId}") { entry ->
                    SessionRoute(
                        module = module,
                        nav = nav,
                        sessionId = entry.arguments?.getString("sessionId"),
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

/** Shows each message from [AppModule.messages] in the snackbar host, and runs its action. */
@Composable
private fun SnackbarMessages(
    module: AppModule,
    snackbars: SnackbarHostState,
) {
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
    var showArchived by rememberSaveable { mutableStateOf(false) }
    val reloads = remember { mutableIntStateOf(0) }
    val lists by produceState(
        DrawerLists.Empty,
        drawerState.currentValue,
        reloads.intValue,
    ) {
        value =
            withContext(Dispatchers.IO) {
                DrawerLists(
                    active = module.home.sessions.sessions(),
                    archived = module.home.sessions.archivedSessions(),
                )
            }
    }
    val themeMode by module.themeMode.collectAsStateWithLifecycle()
    val now = remember(drawerState.currentValue) { System.currentTimeMillis() }
    val rename = sessionRenameViewModel(module.sessionGateway)
    val scope = rememberCoroutineScope()
    val restored = stringResource(R.string.drawer_archived_restored)
    val restoreFailed = stringResource(R.string.drawer_archived_restore_failed)
    HomeDrawer(
        sessions = lists.active,
        archived = lists.archived,
        showArchived = showArchived,
        onToggleArchived = { showArchived = !showArchived },
        onUnarchive = { session ->
            scope.launch {
                if (unarchiveSession(module, session.id)) {
                    reloads.intValue++
                    module.sessionChanges.notifySessionsChanged()
                    module.messages.show(restored)
                } else {
                    module.messages.show(restoreFailed)
                }
            }
        },
        query = query,
        onQueryChange = { query = it },
        currentRoute = currentRoute,
        attentionCount = attentionCount,
        themeMode = themeMode,
        onToggleTheme = { module.theme.cycle() },
        onDestination = { route -> nav.closeThen { nav.goTopLevel(route) } },
        onOpenSession = { nav.closeThen { nav.open("session/${Uri.encode(it)}") } },
        onRenameSession = { rename.start(it.id, it.title) },
        onNewSession = { nav.closeThen { nav.go(Route.NewSession) } },
        onOpenSettings = { nav.closeThen { nav.go(Route.Settings) } },
        onOpenSkills = { nav.closeThen { nav.open(SKILLS_MEMORY_ROUTE) } },
        now = now,
        onSearchMessages = { query -> nav.closeThen { nav.open(searchPath(query)) } },
    )
    SessionRenameHost(rename = rename, onRenamed = { reloads.intValue++ })
}

/** The drawer's two session lists, loaded together. */
private data class DrawerLists(
    val active: List<SessionSummary>,
    val archived: List<SessionSummary>,
) {
    companion object {
        val Empty = DrawerLists(emptyList(), emptyList())
    }
}

/** Restores an archived session. False when the core refuses; the caller shows the failure. */
private suspend fun unarchiveSession(
    module: AppModule,
    sessionId: String,
): Boolean =
    withContext(Dispatchers.IO) {
        try {
            module.sessionGateway.archive(sessionId, false)
            true
        } catch (_: SessionSettingsException) {
            false
        }
    }
