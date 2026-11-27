package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.home.ProjectPickerSheet
import app.tether.home.ProjectSummary
import app.tether.runs.RunsScreen
import app.tether.runs.RunsViewModel
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Registers the full Runs destination from the drawer. */
internal fun NavGraphBuilder.runsRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.Runs.path) {
        RunsRoute(module = module, nav = nav)
    }
}

@Composable
private fun RunsRoute(
    module: AppModule,
    nav: TetherNav,
) {
    val projects by produceState<List<ProjectSummary>?>(null) {
        value =
            withContext(Dispatchers.IO) {
                try {
                    module.home.projects
                        .projects()
                        .sortedByDescending { it.lastActivityAt }
                } catch (cancelled: CancellationException) {
                    throw cancelled
                } catch (
                    @Suppress("TooGenericExceptionCaught") _: Exception,
                ) {
                    emptyList()
                }
            }
    }
    var selectedId by rememberSaveable { mutableStateOf<String?>(null) }
    var pickerOpen by rememberSaveable { mutableStateOf(false) }
    val model: RunsViewModel =
        viewModel(
            factory = RunsViewModel.factory(module.runsGateway, module.executorsGateway),
        )
    LaunchedEffect(projects) {
        if (selectedId == null) selectedId = projects?.firstOrNull()?.id
    }
    LaunchedEffect(model, selectedId) { model.selectProject(selectedId) }
    val projectName = projects?.firstOrNull { it.id == selectedId }?.name
    val context = LocalContext.current
    RunsScreen(
        viewModel = model,
        projectName = projectName,
        onBack = nav::back,
        onOpenProjectPicker = { pickerOpen = true },
        onOpenLocal = { sessionId -> nav.open("session/${Uri.encode(sessionId)}") },
        onOpenCi = { url -> openHttpsUrl(context, url) },
    )
    if (pickerOpen && projects != null) {
        ProjectPickerSheet(
            projects = projects.orEmpty(),
            selectedId = selectedId,
            onSelect = { id ->
                selectedId = id
                pickerOpen = false
            },
            onDismiss = { pickerOpen = false },
        )
    }
}
