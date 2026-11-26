package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.R
import app.tether.design.Archive
import app.tether.design.History
import app.tether.design.Pencil
import app.tether.design.Share2
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.Trash2
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSpace
import app.tether.preview.INDEX_FILE
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/** The session menu's state. [checkpoints] is the edit batch count, or null while it is unknown. */
data class SessionMenuState(
    val checkpoints: Int? = null,
    val archiving: Boolean = false,
    val failure: SessionFailure? = null,
) {
    /** Archive and Delete are refused while a turn runs, so their rows are off then. */
    fun canArchive(running: Boolean): Boolean = !running && !archiving
}

/**
 * Reads the edit batch count for the menu, and archives the session on request. [onArchived] runs
 * after the core has archived it. The menu checks [SessionSheetDeps.isRunning] before it sends.
 */
class SessionMenuViewModel(
    private val deps: SessionSheetDeps,
    private val onArchived: () -> Unit,
    private val io: CoroutineDispatcher = Dispatchers.IO,
) : ViewModel() {
    private val mutableState = MutableStateFlow(SessionMenuState())

    /** The screen state. */
    val state: StateFlow<SessionMenuState> = mutableState.asStateFlow()

    init {
        viewModelScope.launch {
            sessionCall(io) { deps.settings.checkpoints(deps.sessionId).size }
                .onSuccess { count -> mutableState.update { it.copy(checkpoints = count) } }
        }
    }

    /** Hides the session from the lists. A turn running or an archive in progress does nothing. */
    fun archive() {
        if (deps.isRunning() || mutableState.value.archiving) return
        mutableState.update { it.copy(archiving = true, failure = null) }
        viewModelScope.launch {
            sessionCall(io) { deps.settings.archive(deps.sessionId, true) }
                .onSuccess { onArchived() }
                .onFailure { failure ->
                    mutableState.update {
                        it.copy(archiving = false, failure = failure.sessionFailure())
                    }
                }
        }
    }

    companion object {
        /** The menu's view model, for the session described by [deps]. */
        fun factory(
            deps: SessionSheetDeps,
            onArchived: () -> Unit,
        ) = viewModelFactory { initializer { SessionMenuViewModel(deps, onArchived) } }
    }
}

/**
 * Restores an archived session to the lists. It runs on the IO dispatcher and returns the core's
 * result, so the caller can say whether it worked.
 */
suspend fun restoreArchivedSession(
    gateway: SessionGateway,
    sessionId: String,
): Result<Unit> = sessionCall(Dispatchers.IO) { gateway.archive(sessionId, false) }

/** The session menu: Rename, Export, Checkpoints, Preview, Archive, and Delete, in that order. */
@Composable
fun SessionMenuSheet(
    state: SessionMenuState,
    running: Boolean,
    onRename: () -> Unit,
    onExport: () -> Unit,
    onCheckpoints: () -> Unit,
    onPreview: () -> Unit,
    onArchive: () -> Unit,
    onDelete: () -> Unit,
    onDismiss: () -> Unit,
) {
    val canArchive = state.canArchive(running)
    val busyReason = if (running) stringResource(R.string.session_menu_busy) else null
    TetherSheet(title = stringResource(R.string.session_menu_title), onDismiss = onDismiss) {
        Column(
            modifier = Modifier.padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            ListGroup {
                ListRow(
                    title = stringResource(R.string.session_menu_rename),
                    leading = { MenuIcon(TetherIcons.Pencil, TetherTheme.colors.text2) },
                    onClick = onRename,
                )
                ListRow(
                    title = stringResource(R.string.session_menu_export),
                    leading = { MenuIcon(TetherIcons.Share2, TetherTheme.colors.text2) },
                    onClick = onExport,
                )
                ListRow(
                    title = stringResource(R.string.session_menu_checkpoints),
                    subtitle = checkpointSubtitle(state.checkpoints),
                    leading = { MenuIcon(TetherIcons.History, TetherTheme.colors.text2) },
                    onClick = onCheckpoints,
                )
                ListRow(
                    title = stringResource(R.string.session_menu_preview),
                    subtitle = INDEX_FILE,
                    leading = { MenuIcon(TetherIcons.Eye, TetherTheme.colors.text2) },
                    onClick = onPreview,
                )
                ListRow(
                    title = stringResource(R.string.session_menu_archive),
                    subtitle = busyReason,
                    leading = { MenuIcon(TetherIcons.Archive, TetherTheme.colors.text2) },
                    onClick = onArchive,
                    enabled = canArchive,
                )
                ListRow(
                    title = stringResource(R.string.session_menu_delete),
                    subtitle = busyReason,
                    leading = { MenuIcon(TetherIcons.Trash2, TetherTheme.colors.danger) },
                    onClick = onDelete,
                    enabled = canArchive,
                    danger = true,
                )
            }
            if (state.failure != null) {
                InlineBanner(
                    text = stringResource(R.string.session_error_settings),
                    tone = BannerTone.Danger,
                )
            }
        }
    }
}

/** "N edit batches", "No edit batches yet", or null while the count is unknown. */
@Composable
private fun checkpointSubtitle(count: Int?): String? =
    when {
        count == null -> null
        count == 0 -> stringResource(R.string.session_menu_no_batches)
        else -> pluralStringResource(R.plurals.session_menu_batches, count, count)
    }

@Composable
private fun MenuIcon(
    icon: ImageVector,
    tint: Color,
) {
    TetherIcon(icon = icon, contentDescription = null, tint = tint)
}

/**
 * The rename sheet: one field. The title is trimmed before it is saved, and a blank title cannot
 * be saved.
 */
@Composable
fun RenameSheet(
    state: RenameState,
    onTitleChange: (String) -> Unit,
    onSave: () -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.session_rename_title), onDismiss = onDismiss) {
        Column(
            modifier = Modifier.padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            OutlinedTextField(
                value = state.title,
                onValueChange = onTitleChange,
                singleLine = true,
                label = { Text(stringResource(R.string.session_rename_label)) },
                modifier = Modifier.fillMaxWidth(),
            )
            if (state.failure != null) {
                InlineBanner(
                    text = stringResource(R.string.session_error_settings),
                    tone = BannerTone.Danger,
                )
            }
            PrimaryButton(
                label = stringResource(R.string.session_rename_save),
                onClick = onSave,
                modifier = Modifier.fillMaxWidth(),
                enabled = state.canSave,
                loading = state.saving,
            )
        }
    }
}
