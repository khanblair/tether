package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.session.SearchScreen
import app.tether.session.SearchViewModel
import app.tether.session.SessionSettingsGateway

/** The search route. Both arguments are optional; an empty project means no project is in view. */
const val SEARCH_ROUTE = "search?query={query}&project={project}"

/** The search path for [query], with [projectId] when a project is in view. */
fun searchPath(
    query: String,
    projectId: String? = null,
): String = "search?query=${Uri.encode(query)}&project=${Uri.encode(projectId.orEmpty())}"

/** The session sheets' own routes: message search. The session sheets open inside the session. */
fun NavGraphBuilder.sessionSheetRoutes(
    settings: SessionSettingsGateway,
    onBack: () -> Unit,
    onOpenSession: (String) -> Unit,
) {
    composable(
        route = SEARCH_ROUTE,
        arguments =
            listOf(
                navArgument(QUERY_ARG) {
                    type = NavType.StringType
                    defaultValue = ""
                },
                navArgument(PROJECT_ARG) {
                    type = NavType.StringType
                    defaultValue = ""
                },
            ),
    ) { entry ->
        SearchRoute(
            settings = settings,
            query = entry.arguments?.getString(QUERY_ARG).orEmpty(),
            projectId = entry.arguments?.getString(PROJECT_ARG).orEmpty(),
            onBack = onBack,
            onOpenSession = onOpenSession,
        )
    }
}

@Composable
private fun SearchRoute(
    settings: SessionSettingsGateway,
    query: String,
    projectId: String,
    onBack: () -> Unit,
    onOpenSession: (String) -> Unit,
) {
    val project = projectId.ifBlank { null }
    val viewModel: SearchViewModel =
        viewModel(factory = SearchViewModel.factory(settings, project, query))
    SearchScreen(
        viewModel = viewModel,
        hasProject = project != null,
        onBack = onBack,
        onOpenSession = onOpenSession,
    )
}

private const val QUERY_ARG = "query"
private const val PROJECT_ARG = "project"
