package app.tether.nav

import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.di.AppModule
import app.tether.session.StartResult
import app.tether.session.compose.CommandScreen
import kotlinx.coroutines.CancellationException
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
            CommandScreen.Linux -> linuxPath(projectId)
            CommandScreen.Context, CommandScreen.Help -> null
        }
    if (path == null) missing() else nav.open(path)
}

/** The messages a session start shows when it cannot finish. */
internal class StartMessages(
    val noKey: String,
    val addKey: String,
    val noModels: String,
    val openProviders: String,
    val failed: String,
)

/** What a start that did not open a session tells the person, and the action it offers. */
internal data class StartNotice(
    val text: String,
    val actionLabel: String?,
)

/** The notice for [result], or null when the session started. No key or model offers Providers. */
internal fun StartMessages.noticeFor(result: StartResult): StartNotice? =
    when (result) {
        is StartResult.Started -> null
        StartResult.NoKey -> StartNotice(noKey, addKey)
        StartResult.NoModels -> StartNotice(noModels, openProviders)
        StartResult.Failed -> StartNotice(failed, null)
    }

/** The start messages from the string resources. */
@Composable
internal fun rememberStartMessages(): StartMessages {
    val noKey = stringResource(R.string.project_home_no_key)
    val addKey = stringResource(R.string.project_home_add_key)
    val noModels = stringResource(R.string.project_home_no_models)
    val openProviders = stringResource(R.string.project_home_open_providers)
    val failed = stringResource(R.string.project_home_start_failed)
    return remember(noKey, addKey, noModels, openProviders, failed) {
        StartMessages(noKey, addKey, noModels, openProviders, failed)
    }
}

/**
 * Starts a session in the project with [projectId] and opens the real session screen. A start
 * that cannot finish shows the message that fits it, with a way to Providers when a key or a
 * model is missing, and stays where it is.
 */
internal fun startNewSession(
    module: AppModule,
    nav: TetherNav,
    projectId: String?,
    scope: CoroutineScope,
    messages: StartMessages,
) {
    if (projectId == null) {
        module.messages.show(messages.failed)
        return
    }
    scope.launch {
        val result = startSafely(module, projectId)
        val notice = messages.noticeFor(result)
        if (result is StartResult.Started) {
            nav.open("session/${Uri.encode(result.sessionId)}")
        } else if (notice != null) {
            module.messages.show(
                text = notice.text,
                actionLabel = notice.actionLabel,
                onAction = notice.actionLabel?.let { { nav.go(Route.Providers) } },
            )
        }
    }
}

@Suppress("TooGenericExceptionCaught")
private suspend fun startSafely(
    module: AppModule,
    projectId: String,
): StartResult =
    withContext(Dispatchers.IO) {
        try {
            module.sessionStarter.start(projectId)
        } catch (cancelled: CancellationException) {
            throw cancelled
        } catch (_: Exception) {
            StartResult.Failed
        }
    }
