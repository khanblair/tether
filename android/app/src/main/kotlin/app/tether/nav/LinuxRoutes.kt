package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.NavType
import androidx.navigation.compose.composable
import androidx.navigation.navArgument
import app.tether.di.AppModule
import app.tether.linux.LinuxShellScreen
import app.tether.linux.LinuxShellViewModel
import app.tether.linux.LinuxUnavailableScreen
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The Linux shell route, using the requested project or the most recently active project. */
internal fun NavGraphBuilder.linuxRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(
        route = "${Route.Linux.path}?projectId={projectId}",
        arguments =
            listOf(
                navArgument("projectId") {
                    type = NavType.StringType
                    nullable = true
                    defaultValue = null
                },
            ),
    ) { entry ->
        val context = LocalContext.current
        val requested = entry.arguments?.getString("projectId")
        LinuxProject(module, requested) { projectId, projectName ->
            if (!module.linuxSetup.executorBuildConfig(context).linuxSupported) {
                LinuxUnavailableScreen(onBack = nav::back)
            } else {
                val model: LinuxShellViewModel =
                    viewModel(
                        key = projectId.orEmpty(),
                        factory = LinuxShellViewModel.factory(module.linuxShellGateway),
                    )
                androidx.compose.runtime.LaunchedEffect(projectId) {
                    model.open(projectId, TerminalRows.DEFAULT, TerminalRows.DEFAULT_COLUMNS)
                }
                LinuxShellScreen(
                    viewModel = model,
                    projectName = projectName,
                    onBack = nav::back,
                    onOpenExecutors = { nav.open(SETTINGS_EXECUTORS_ROUTE) },
                    onClose = {
                        model.closeShell()
                        nav.back()
                    },
                    messages = module.messages,
                )
            }
        }
    }
}

/** Opens Linux with a session's project without adding a second route entry. */
internal fun linuxPath(projectId: String?): String =
    if (projectId == null) {
        Route.Linux.path
    } else {
        "${Route.Linux.path}?projectId=${Uri.encode(projectId)}"
    }

@Composable
private fun LinuxProject(
    module: AppModule,
    requestedId: String?,
    content: @Composable (String?, String?) -> Unit,
) {
    val result by produceState<ProjectSelection?>(null, requestedId) {
        value =
            withContext(Dispatchers.IO) {
                val projects =
                    try {
                        module.home.projects.projects()
                    } catch (error: CancellationException) {
                        throw error
                    } catch (_: Exception) {
                        emptyList()
                    }
                val selected = requestedId ?: projects.firstOrNull()?.id
                ProjectSelection(selected, projects.firstOrNull { it.id == selected }?.name)
            }
    }
    result?.let { content(it.id, it.name) }
}

private data class ProjectSelection(
    val id: String?,
    val name: String?,
)

private object TerminalRows {
    const val DEFAULT = 24
    const val DEFAULT_COLUMNS = 80
}
