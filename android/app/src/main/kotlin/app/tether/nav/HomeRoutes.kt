package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.home.AttentionItem
import app.tether.home.AttentionScreen
import app.tether.home.DashboardScreen
import app.tether.home.DashboardViewModel
import app.tether.home.NewProjectChoice
import app.tether.home.NewProjectScreen
import app.tether.home.NewProjectViewModel
import app.tether.home.ProjectHomeScreen
import app.tether.home.ProjectHomeViewModel
import app.tether.home.ProjectsScreen
import app.tether.home.ProjectsViewModel
import app.tether.home.SessionRenameHost
import app.tether.home.sessionRenameViewModel

/** The start screens: Home, Projects, and Needs attention. */
internal fun NavGraphBuilder.homeRoutes(
    module: AppModule,
    nav: TetherNav,
    attentionItems: List<AttentionItem>,
) {
    dashboardRoute(module, nav, attentionItems)
    projectRoutes(module, nav)
    attentionRoute(nav, attentionItems)
}

/** Home, the start screen. */
private fun NavGraphBuilder.dashboardRoute(
    module: AppModule,
    nav: TetherNav,
    attentionItems: List<AttentionItem>,
) {
    composable(Route.Home.path) {
        val dashboard: DashboardViewModel =
            viewModel(
                factory =
                    DashboardViewModel.factory(
                        module.home,
                        module.providerGateway,
                        module.updates.state,
                    ),
            )
        val rename = sessionRenameViewModel(module.sessionGateway)
        DashboardScreen(
            viewModel = dashboard,
            attentionCount = attentionItems.size,
            onOpenMenu = nav::openDrawer,
            onOpenAttention = { nav.go(Route.Attention) },
            onOpenSession = { nav.open("session/${Uri.encode(it)}") },
            onOpenAbout = { nav.go(Route.About) },
            onOpenProject = { nav.open(projectHomePath(it)) },
            onOpenProjects = { nav.goTopLevel(Route.Projects) },
            onOpenProviders = { nav.go(Route.Providers) },
            onNewProject = { choice -> nav.go(choice.route()) },
            onRenameSession = { rename.start(it.id, it.title) },
        )
        SessionRenameHost(rename = rename, onRenamed = dashboard::refresh)
    }
}

/** The project list and a single project's home. */
private fun NavGraphBuilder.projectRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.Projects.path) {
        val projects: ProjectsViewModel =
            viewModel(factory = ProjectsViewModel.factory(module.home.projects))
        ProjectsScreen(
            viewModel = projects,
            onOpenMenu = nav::openDrawer,
            onOpenProject = { nav.open(projectHomePath(it)) },
            onNewProject = { choice -> nav.go(choice.route()) },
        )
    }
    projectHomeRoute(module, nav)
    newProjectRoute(module, nav)
}

/** One project's home, with its sessions and the New session button. */
private fun NavGraphBuilder.projectHomeRoute(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Routes.PROJECT_HOME) { entry ->
        val id = entry.arguments?.getString("projectId").orEmpty()
        val home: ProjectHomeViewModel =
            viewModel(
                key = id,
                factory =
                    ProjectHomeViewModel.factory(
                        id,
                        module.home.projects,
                        module.home.sessions,
                        module.sessionStarter,
                        module.providerGateway,
                    ),
            )
        val state by home.state.collectAsStateWithLifecycle()
        LifecycleResumeEffect(home) {
            home.refresh()
            onPauseOrDispose {}
        }
        LaunchedEffect(home) {
            home.openedSessions.collect { nav.open("session/${Uri.encode(it)}") }
        }
        var exporting by rememberSaveable { mutableStateOf(false) }
        val rename = sessionRenameViewModel(module.sessionGateway)
        ProjectHomeScreen(
            state = state,
            onBack = nav::back,
            onOpenSession = { nav.open("session/${Uri.encode(it)}") },
            onNewSession = home::startSession,
            onTestConnection = home::testConnection,
            onOpenProviders = { nav.go(Route.Providers) },
            onExport = { exporting = true },
            onOpenChanges = { nav.open(gitPath(id)) },
            onOpenFiles = { nav.open(filesPath(id)) },
            onRenameSession = { rename.start(it.id, it.title) },
        )
        SessionRenameHost(rename = rename, onRenamed = home::refresh)
        val project = state.project
        if (exporting && project != null) {
            ExportProjectHost(
                module = module,
                projectId = id,
                projectName = project.name,
                onDismiss = { exporting = false },
            )
        }
    }
}

/** New empty project. Create replaces this screen with the new project's home. */
private fun NavGraphBuilder.newProjectRoute(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.NewProject.path) {
        val create: NewProjectViewModel =
            viewModel(factory = NewProjectViewModel.factory(module.projectGateway))
        val state by create.state.collectAsStateWithLifecycle()
        NewProjectScreen(
            state = state,
            onBack = nav::back,
            onNameChange = create::onNameChange,
            onCreate = create::create,
            onCreated = { id -> nav.replace(projectHomePath(id), Route.NewProject) },
        )
    }
}

/** Needs attention, the list of sessions that wait on the user. */
private fun NavGraphBuilder.attentionRoute(
    nav: TetherNav,
    attentionItems: List<AttentionItem>,
) {
    composable(Route.Attention.path) {
        AttentionScreen(
            items = attentionItems,
            onOpenSession = { nav.open("session/${Uri.encode(it)}") },
            onOpenAbout = { nav.go(Route.About) },
            onBack = nav::back,
        )
    }
}

/** The route for each new-project choice. */
internal fun NewProjectChoice.route(): Route =
    when (this) {
        NewProjectChoice.Clone -> Route.CloneProject
        NewProjectChoice.Empty -> Route.NewProject
        NewProjectChoice.Import -> Route.ImportProject
    }
