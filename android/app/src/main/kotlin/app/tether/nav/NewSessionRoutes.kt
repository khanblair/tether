package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.R
import app.tether.di.AppModule
import app.tether.newsession.NewSessionScreen
import app.tether.newsession.NewSessionStartError
import app.tether.newsession.NewSessionUiState
import app.tether.newsession.NewSessionViewModel

/** The New session screen, with an optional project override. */
internal fun NavGraphBuilder.newSessionRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(
        route = "${Route.NewSession.path}?projectId={projectId}",
        arguments =
            listOf(
                navArgument("projectId") {
                    type = NavType.StringType
                    nullable = true
                    defaultValue = null
                },
            ),
    ) { entry ->
        val requested = entry.arguments?.getString("projectId")
        val model: NewSessionViewModel =
            viewModel(
                key = requested.orEmpty(),
                factory =
                    NewSessionViewModel.factory(
                        projects = module.home.projects,
                        previews = module.newSessionGateway,
                        starter = module.sessionStarter,
                        pending = module.pendingFirstMessage,
                        requestedProjectId = requested,
                    ),
            )
        val state by model.state.collectAsStateWithLifecycle()
        NewSessionEffects(module = module, nav = nav, model = model, state = state)
        NewSessionScreen(
            state = state,
            onBack = nav::back,
            onSelectProject = model::selectProject,
            onSend = model::send,
            onNewProject = { nav.go(Route.NewProject) },
            onRetry = model::retry,
        )
    }
}

/** Creates the route with a project selected when a caller already has one. */
internal fun newSessionPath(projectId: String?): String =
    projectId?.let {
        "${Route.NewSession.path}?projectId=${Uri.encode(it)}"
    } ?: Route.NewSession.path

@Composable
private fun NewSessionEffects(
    module: AppModule,
    nav: TetherNav,
    model: NewSessionViewModel,
    state: NewSessionUiState,
) {
    val noKey = stringResource(R.string.project_home_no_key)
    val addKey = stringResource(R.string.project_home_add_key)
    val noModels = stringResource(R.string.project_home_no_models)
    val openProviders = stringResource(R.string.project_home_open_providers)
    val failed = stringResource(R.string.project_home_start_failed)
    LaunchedEffect(model) {
        model.openedSessions.collect { sessionId ->
            nav.replace("session/${Uri.encode(sessionId)}", Route.NewSession)
        }
    }
    val error = (state as? NewSessionUiState.Ready)?.startError
    LaunchedEffect(error) {
        when (error) {
            NewSessionStartError.NoKey ->
                module.messages.show(
                    text = noKey,
                    actionLabel = addKey,
                    onAction = { nav.go(Route.Providers) },
                )
            NewSessionStartError.NoModels ->
                module.messages.show(
                    text = noModels,
                    actionLabel = openProviders,
                    onAction = { nav.go(Route.Providers) },
                )
            NewSessionStartError.Failed ->
                module.messages.show(failed)
            null -> Unit
        }
    }
}
