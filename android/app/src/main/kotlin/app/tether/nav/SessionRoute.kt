package app.tether.nav

import android.util.Log
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
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

/** A session's transcript. The connection opens off the main thread and closes when left. */
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
    var connection by remember(sessionId) { mutableStateOf<SessionConnection?>(null) }
    var initialMessage by rememberSaveable(sessionId) { mutableStateOf<String?>(null) }
    var unavailable by remember(sessionId) { mutableStateOf(sessionId == null) }
    var reason by remember(sessionId) { mutableStateOf<String?>(null) }
    LaunchedEffect(sessionId) {
        if (sessionId != null) {
            when (val result = openSession(module, sessionId)) {
                is OpenResult.Opened -> {
                    connection = result.connection
                    if (initialMessage == null) {
                        initialMessage = module.pendingFirstMessage.take(sessionId)
                    }
                }
                is OpenResult.Unavailable -> {
                    reason = result.reason
                    unavailable = true
                }
            }
        }
    }
    DisposableEffect(connection) {
        val opened = connection
        onDispose { opened?.let { module.closeInBackground(it::close) } }
    }
    val projectId = connection?.projectId
    if (unavailable) {
        SessionUnavailableScreen(onBack = onBack, reason = reason)
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

/** What opening a session gave back: the connection, or the reason it could not open. */
private sealed interface OpenResult {
    class Opened(
        val connection: SessionConnection,
    ) : OpenResult

    /**
     * [reason] is the exception's simple class name. It is null when the core reports the session
     * as unavailable, which is the same case as before.
     */
    class Unavailable(
        val reason: String?,
    ) : OpenResult
}

/**
 * Opens the connection off the main thread. A [SessionRuntimeException] means the session cannot be
 * opened. Any other failure, such as a native panic, is logged by its class name and shown the same
 * way, so the app does not end. A cancellation is rethrown.
 */
private suspend fun openSession(
    module: AppModule,
    sessionId: String,
): OpenResult =
    withContext(Dispatchers.IO) {
        try {
            OpenResult.Opened(
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
            OpenResult.Unavailable(reason = null)
        } catch (
            @Suppress("TooGenericExceptionCaught") error: Exception,
        ) {
            val name = error.javaClass.simpleName
            Log.w(TAG, "session open failed: $name")
            OpenResult.Unavailable(reason = name)
        }
    }

private const val TAG = "Tether"
