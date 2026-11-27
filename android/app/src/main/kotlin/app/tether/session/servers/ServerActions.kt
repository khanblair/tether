package app.tether.session.servers

import app.tether.core.generated.SessionRuntimeException
import app.tether.nav.serverPreviewKey
import app.tether.session.SessionConnection
import app.tether.session.SessionSheetEnv
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

internal fun serverPreviewAction(sheets: SessionSheetEnv?): (ServerPreviewTarget) -> Unit =
    { target ->
        val open = sheets?.openPreview
        if (open != null) {
            when (target) {
                is ServerPreviewTarget.File -> open(target.path)
                is ServerPreviewTarget.Server ->
                    open(
                        serverPreviewKey(target.url, target.processId),
                    )
            }
        }
    }

internal fun serverStopAction(
    scope: CoroutineScope,
    connection: SessionConnection?,
    onFailure: () -> Unit,
): (String) -> Unit =
    { processId ->
        val active = connection
        if (active != null) {
            scope.launch(Dispatchers.IO) {
                try {
                    active.stopProcess(processId)
                } catch (_: SessionRuntimeException) {
                    onFailure()
                }
            }
        }
    }
