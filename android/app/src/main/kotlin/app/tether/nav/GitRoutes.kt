package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.di.AppModule
import app.tether.git.DiffScreen
import app.tether.git.DiffViewModel
import app.tether.git.GitScreen
import app.tether.git.GitViewModel

private const val GIT_ROUTE = "git/{projectId}"
private const val GIT_DIFF_ROUTE = "git/{projectId}/diff?staged={staged}&path={path}"

/** The Git screen of the project with [projectId]: changes, branches, and history. */
fun gitPath(projectId: String) = "git/${Uri.encode(projectId)}"

/** The diff of [path], or of every change when [path] is null, on the staged or unstaged side. */
fun gitDiffPath(
    projectId: String,
    path: String?,
    staged: Boolean,
): String {
    val file = path?.let { "&path=${Uri.encode(it)}" }.orEmpty()
    return "git/${Uri.encode(projectId)}/diff?staged=$staged$file"
}

/** The Git screen of a project and its diff screen. */
internal fun NavGraphBuilder.gitRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    gitScreenRoute(module, nav)
    gitDiffRoute(module, nav)
}

private fun NavGraphBuilder.gitScreenRoute(
    module: AppModule,
    nav: TetherNav,
) {
    composable(
        GIT_ROUTE,
        arguments = listOf(navArgument("projectId") { type = NavType.StringType }),
    ) { entry ->
        val id = entry.arguments?.getString("projectId").orEmpty()
        val git: GitViewModel =
            viewModel(
                key = id,
                factory = viewModelFactory { initializer { GitViewModel(id, module.gitGateway) } },
            )
        val state by git.state.collectAsStateWithLifecycle()
        GitScreen(
            state = state,
            onSelectTab = git::selectTab,
            onRefresh = git::refresh,
            onOpenDiff = { path, staged -> nav.open(gitDiffPath(id, path, staged)) },
            onBack = nav::back,
        )
    }
}

private fun NavGraphBuilder.gitDiffRoute(
    module: AppModule,
    nav: TetherNav,
) {
    composable(
        GIT_DIFF_ROUTE,
        arguments =
            listOf(
                navArgument("projectId") { type = NavType.StringType },
                navArgument("staged") {
                    type = NavType.BoolType
                    defaultValue = false
                },
                navArgument("path") {
                    type = NavType.StringType
                    nullable = true
                    defaultValue = null
                },
            ),
    ) { entry ->
        val id = entry.arguments?.getString("projectId").orEmpty()
        val path = entry.arguments?.getString("path")
        val staged = entry.arguments?.getBoolean("staged") ?: false
        val diff: DiffViewModel =
            viewModel(
                key = "$id/$path/$staged",
                factory =
                    viewModelFactory {
                        initializer { DiffViewModel(id, path, staged, module.gitGateway) }
                    },
            )
        val state by diff.state.collectAsStateWithLifecycle()
        DiffScreen(
            state = state,
            onSetStaged = diff::setStaged,
            onToggleWrap = diff::toggleWrap,
            onRetry = diff::retry,
            onBack = nav::back,
        )
    }
}
