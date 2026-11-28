package app.tether.nav

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import app.tether.R
import app.tether.di.AppModule
import app.tether.home.ProjectPickerSheet
import app.tether.home.ProjectSummary
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The drawer's New session. It starts a session in the only project, or asks which project first,
 * then opens the real session screen like every other way to start one. With no project it opens
 * New project.
 */
@Stable
internal class NewSessionLauncher(
    private val module: AppModule,
    private val nav: TetherNav,
    private val scope: CoroutineScope,
    private val context: Context,
    private val messages: StartMessages,
) {
    /** The projects to choose from while the picker is open, otherwise null. */
    var choices by mutableStateOf<List<ProjectSummary>?>(null)
        private set

    /** Starts the flow. */
    fun start() {
        scope.launch {
            val projects = recentProjects()
            when {
                projects == null -> module.messages.show(messages.failed)
                projects.isEmpty() -> {
                    module.messages.show(context.getString(R.string.start_session_no_project))
                    nav.go(Route.NewProject)
                }
                projects.size == 1 -> begin(projects.single().id)
                else -> choices = projects
            }
        }
    }

    /** Starts in the project with [projectId], chosen in the picker. */
    fun choose(projectId: String) {
        choices = null
        begin(projectId)
    }

    /** Closes the picker without starting. */
    fun dismiss() {
        choices = null
    }

    private fun begin(projectId: String) = startNewSession(module, nav, projectId, scope, messages)

    @Suppress("TooGenericExceptionCaught")
    private suspend fun recentProjects(): List<ProjectSummary>? =
        withContext(Dispatchers.IO) {
            try {
                module.home.projects
                    .projects()
                    .sortedByDescending(ProjectSummary::lastActivityAt)
            } catch (cancelled: CancellationException) {
                throw cancelled
            } catch (_: Exception) {
                null
            }
        }
}

/** The launcher for the drawer's New session. */
@Composable
internal fun rememberNewSessionLauncher(
    module: AppModule,
    nav: TetherNav,
): NewSessionLauncher {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val messages = rememberStartMessages()
    return remember(module, nav, scope, context, messages) {
        NewSessionLauncher(module, nav, scope, context, messages)
    }
}

/** The project picker the launcher opens when there is more than one project. */
@Composable
internal fun NewSessionPicker(launcher: NewSessionLauncher) {
    val projects = launcher.choices ?: return
    ProjectPickerSheet(
        projects = projects,
        selectedId = null,
        onSelect = launcher::choose,
        onDismiss = launcher::dismiss,
    )
}
