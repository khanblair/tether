package app.tether.nav

import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.home.AttentionItem
import app.tether.home.AttentionScreen
import app.tether.home.DashboardScreen
import app.tether.home.DashboardViewModel
import app.tether.home.NewProjectChoice
import app.tether.home.ProjectsScreen
import app.tether.home.ProjectsViewModel

/** The start screens: Home, Projects, and Needs attention. */
internal fun NavGraphBuilder.homeRoutes(
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
        DashboardScreen(
            viewModel = dashboard,
            attentionCount = attentionItems.size,
            onOpenMenu = nav::openDrawer,
            onOpenAttention = { nav.go(Route.Attention) },
            onOpenSession = { nav.go(Route.Session) },
            onOpenAbout = { nav.go(Route.About) },
            onOpenProject = { nav.go(Route.ProjectHome) },
            onOpenProjects = { nav.goTopLevel(Route.Projects) },
            onOpenProviders = { nav.go(Route.Providers) },
            onNewProject = { choice -> nav.go(choice.route()) },
        )
    }
    composable(Route.Projects.path) {
        val projects: ProjectsViewModel =
            viewModel(factory = ProjectsViewModel.factory(module.home.projects))
        ProjectsScreen(
            viewModel = projects,
            onOpenMenu = nav::openDrawer,
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

/** The route for each new-project choice. */
internal fun NewProjectChoice.route(): Route =
    when (this) {
        NewProjectChoice.Clone -> Route.CloneProject
        NewProjectChoice.Empty -> Route.NewProject
        NewProjectChoice.Import -> Route.ImportProject
    }
