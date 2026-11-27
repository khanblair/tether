package app.tether.nav

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.settings.executors.CiRunsScreen
import app.tether.settings.executors.CiRunsViewModel
import app.tether.settings.executors.ExecutorsScreen
import app.tether.settings.executors.ExecutorsViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The CI runs list, opened from the Executors screen. */
const val SETTINGS_CI_RUNS_ROUTE = "settings/executors/runs"

/** The Executors settings and the CI runs of the most recently active project. */
internal fun NavGraphBuilder.executorsRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_EXECUTORS_ROUTE) {
        val context = LocalContext.current
        WithActiveProject(module) { projectId ->
            ExecutorsScreen(
                viewModel =
                    viewModel(
                        key = projectId.orEmpty(),
                        factory =
                            ExecutorsViewModel.factory(
                                module.executorsGateway,
                                module.linuxSetup.executorBuildConfig(context),
                                projectId,
                                linuxBackground = module.linuxBackground,
                                onLinuxReadyChanged = module.linuxActivity::reportLinuxReady,
                            ),
                    ),
                onBack = nav::back,
                onOpenCiRuns = { nav.open(SETTINGS_CI_RUNS_ROUTE) },
                onOpenLinux = { nav.open(linuxPath(projectId)) },
            )
        }
    }
    composable(SETTINGS_CI_RUNS_ROUTE) {
        val context = LocalContext.current
        WithActiveProject(module) { projectId ->
            CiRunsScreen(
                viewModel =
                    viewModel(
                        key = projectId.orEmpty(),
                        factory = CiRunsViewModel.factory(module.executorsGateway, projectId),
                    ),
                onBack = nav::back,
                onOpenUrl = { url -> openHttpsUrl(context, url) },
            )
        }
    }
}

/** The project id found on the first read. A null [id] means no project exists yet. */
private data class ActiveProject(
    val id: String?,
)

/**
 * Reads the most recently active project off the main thread, then shows [content] with its id,
 * or with null when there is no project. Nothing shows while the read runs.
 */
@Composable
private fun WithActiveProject(
    module: AppModule,
    content: @Composable (String?) -> Unit,
) {
    val active by produceState<ActiveProject?>(null) {
        value =
            withContext(Dispatchers.IO) {
                ActiveProject(
                    runCatching {
                        module.projectGateway
                            .list()
                            .firstOrNull()
                            ?.id
                    }.getOrNull(),
                )
            }
    }
    active?.let { content(it.id) }
}
