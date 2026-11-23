package app.tether.nav

import android.net.Uri
import app.tether.di.AppModule
import app.tether.session.StartResult
import app.tether.session.compose.CommandScreen
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Opens the screen a slash command names, for the session in the project with [projectId]. A
 * screen the app does not have yet calls [missing] instead.
 */
internal fun openCommandScreen(
    nav: TetherNav,
    projectId: String?,
    target: CommandScreen,
    arguments: String,
    missing: () -> Unit,
) {
    val path =
        when (target) {
            CommandScreen.Diff -> projectId?.let { gitDiffPath(it, null, false) }
            CommandScreen.CiRuns -> SETTINGS_CI_RUNS_ROUTE
            CommandScreen.Cost -> Route.Cost.path
            CommandScreen.Skills -> SKILLS_ROUTE
            CommandScreen.Memory -> MEMORY_ROUTE
            CommandScreen.Vault -> if (arguments == "lint") VAULT_LINT_ROUTE else VAULT_ROUTE
            CommandScreen.Linux, CommandScreen.Context, CommandScreen.Help -> null
        }
    if (path == null) missing() else nav.open(path)
}

/**
 * Starts another session in the project with [projectId] and opens it. A start that cannot finish
 * calls [failed] and stays on the current session.
 */
internal fun startNewSession(
    module: AppModule,
    nav: TetherNav,
    projectId: String?,
    scope: CoroutineScope,
    failed: () -> Unit,
) {
    if (projectId == null) {
        failed()
        return
    }
    scope.launch {
        val result = withContext(Dispatchers.IO) { module.sessionStarter.start(projectId) }
        if (result is StartResult.Started) {
            nav.open("session/${Uri.encode(result.sessionId)}")
        } else {
            failed()
        }
    }
}
