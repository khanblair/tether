package app.tether.home

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import app.tether.session.RenameSheet
import app.tether.session.RenameState
import app.tether.session.SessionFailure
import app.tether.session.SessionGateway

/**
 * Shows the rename sheet while a rename is open. [onRenamed] runs after a save, so the list behind
 * the sheet reloads.
 */
@Composable
fun SessionRenameHost(
    rename: SessionRenameViewModel,
    onRenamed: () -> Unit,
) {
    val state by rename.state.collectAsStateWithLifecycle()
    if (state.sessionId == null) return
    RenameSheet(
        state =
            RenameState(
                title = state.title,
                saving = state.saving,
                failure = if (state.failed) SessionFailure.OTHER else null,
            ),
        onTitleChange = rename::onTitleChange,
        onSave = { rename.save(onRenamed) },
        onDismiss = rename::dismiss,
    )
}

/** The rename view model for a list screen, over the app's session gateway. */
@Composable
fun sessionRenameViewModel(sessions: SessionGateway): SessionRenameViewModel =
    viewModel(factory = SessionRenameViewModel.factory(sessions))
