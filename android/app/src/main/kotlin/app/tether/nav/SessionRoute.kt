package app.tether.nav

import android.util.Log
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.R
import app.tether.core.generated.SessionRuntimeException
import app.tether.di.AppModule
import app.tether.session.SessionConnection
import app.tether.session.SessionSheetEnv
import app.tether.session.SessionUnavailableScreen
import app.tether.session.TranscriptScreen
import app.tether.session.restoreArchivedSession
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * A session's transcript. The connection opens off the main thread and lives with this back stack
 * entry, so it stays open while a screen such as a server preview is on top of the transcript.
 */
@Composable
internal fun SessionRoute(
    module: AppModule,
    nav: TetherNav,
    sessionId: String?,
) {
    val onBack: () -> Unit = nav::back
    val scope = rememberCoroutineScope()
    val screenMissing = stringResource(R.string.compose_command_screen_missing)
    val newSessionFailed = stringResource(R.string.compose_command_new_failed)
    val changeFailed = stringResource(R.string.session_error_settings)
    val holder: SessionConnectionHolder<SessionConnection> =
        viewModel(
            key = "session/$sessionId",
            factory =
                viewModelFactory {
                    initializer {
                        SessionConnectionHolder(
                            sessionId = sessionId,
                            release = { opened -> module.closeInBackground(opened::close) },
                            launch = module::launchInBackground,
                            open = { id -> openSession(module, id) },
                        )
                    }
                },
        )
    val open by holder.state.collectAsStateWithLifecycle()
    val connection = (open as? SessionOpen.Opened)?.connection
    var initialMessage by rememberSaveable(sessionId) { mutableStateOf<String?>(null) }
    LaunchedEffect(sessionId, connection) {
        if (sessionId != null && connection != null && initialMessage == null) {
            initialMessage = module.pendingFirstMessage.take(sessionId)
        }
    }
    val projectId = connection?.projectId
    val unavailable = open as? SessionOpen.Unavailable
    if (unavailable != null) {
        SessionUnavailableScreen(onBack = onBack, reason = unavailable.reason)
    } else {
        TranscriptScreen(
            connection = connection,
            initialMessage = initialMessage,
            onInitialMessageStarted = { initialMessage = null },
            onBack = onBack,
            sheets =
                SessionSheetEnv(
                    providers = module.providerGateway,
                    messages = module.messages,
                    openProviders = { nav.go(Route.Providers) },
                    files = module.filesGateway,
                    openScreen = { target, arguments ->
                        openCommandScreen(nav, projectId, target, arguments) {
                            module.messages.show(screenMissing)
                        }
                    },
                    newSession = {
                        startNewSession(module, nav, projectId, scope) {
                            module.messages.show(newSessionFailed)
                        }
                    },
                    openDrawer = nav::openDrawer,
                    projects = module.home.projects,
                    leaveSession = onBack,
                    undoArchive = { id ->
                        module.launchInBackground {
                            restoreArchivedSession(module.sessionGateway, id)
                                .onSuccess { module.sessionChanges.notifySessionsChanged() }
                                .onFailure { module.messages.show(changeFailed) }
                        }
                    },
                    sessionsChanged = module.sessionChanges::notifySessionsChanged,
                    openPreview = nav.previewFile(projectId),
                    openInlinePreview = nav.previewInline(module.inlinePreviews),
                ),
            onOpenBudgets = { nav.open(SETTINGS_BUDGETS_ROUTE) },
        )
    }
}

/**
 * Opens the connection off the main thread. A [SessionRuntimeException] means the session cannot be
 * opened. Any other failure, such as a native panic, is logged by its class name and shown the same
 * way, so the app does not end. A cancellation is rethrown.
 */
private suspend fun openSession(
    module: AppModule,
    sessionId: String,
): SessionOpen<SessionConnection> =
    withContext(Dispatchers.IO) {
        try {
            SessionOpen.Opened(
                SessionConnection(
                    module.databasePath,
                    module.vault,
                    sessionId,
                    workspacesRoot = module.workspacesRoot,
                ),
            )
        } catch (error: CancellationException) {
            throw error
        } catch (_: SessionRuntimeException) {
            SessionOpen.Unavailable(reason = null)
        } catch (
            @Suppress("TooGenericExceptionCaught") error: Exception,
        ) {
            val name = error.javaClass.simpleName
            Log.w(TAG, "session open failed: $name")
            SessionOpen.Unavailable(reason = name)
        }
    }

private const val TAG = "Tether"
