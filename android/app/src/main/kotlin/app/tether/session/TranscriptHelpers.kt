package app.tether.session

import app.tether.core.generated.CoreEventView
import app.tether.session.compose.CommandScreen
import app.tether.session.servers.ServerPreviewTarget
import app.tether.session.servers.ServerStateHolder
import app.tether.session.servers.processChange
import app.tether.session.servers.serverPreviewTarget

/** Opens a command screen for this session. Without sheets, the action is missing. */
internal fun screenOpener(
    sheets: SessionSheetEnv?,
    target: CommandScreen,
): (() -> Unit)? = sheets?.let { env -> { env.openScreen(target, "") } }

/** A sheet opener for a session with sheets. Without sheets, the control has no action. */
internal fun sheetOpener(
    sheets: SessionSheetEnv?,
    control: SessionSheetControl,
    kind: SessionSheetKind,
): (() -> Unit)? = sheets?.let { { control.openSheet(kind) } }

/**
 * Handles the server side of one core event. A stopped process shows its exit code. A server that
 * is ready is passed to [onPreviewReady], which offers its preview.
 */
internal fun handleServerEvent(
    event: CoreEventView,
    serverState: ServerStateHolder,
    sheets: SessionSheetEnv?,
    stoppedFormat: String,
    onPreviewReady: (ServerPreviewTarget) -> Unit,
) {
    event.processChange()?.let { change ->
        serverState.onProcessChanged(change)?.let { code ->
            sheets?.messages?.show(stoppedFormat.format(code))
        }
    }
    event.serverPreviewTarget()?.let(onPreviewReady)
}
