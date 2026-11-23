package app.tether.projectarchive

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * The Import a ZIP screen. The route owns the file picker: [onChooseFile] starts it, and the picked
 * file comes back to the view model. [onImported] opens the new project once, when it exists.
 */
@Composable
fun ImportProjectScreen(
    state: ImportProjectUiState,
    onBack: () -> Unit,
    onChooseFile: () -> Unit,
    onNameChange: (String) -> Unit,
    onImport: () -> Unit,
    onCancel: () -> Unit,
    onImported: (String) -> Unit,
) {
    LaunchedEffect(state.projectId) {
        state.projectId?.let(onImported)
    }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.archive_import_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
            modifier = Modifier.fillMaxSize().padding(padding).padding(TetherSpace.space4),
        ) {
            when (state.phase) {
                ImportPhase.Idle -> IdleContent(onChooseFile)
                ImportPhase.Picked,
                ImportPhase.Failed,
                -> PickedContent(state, onChooseFile, onNameChange, onImport)
                ImportPhase.Importing -> ImportingContent(onCancel)
                ImportPhase.Done -> DoneContent()
            }
        }
    }
}

/** The short explanation and the one action on an empty screen. */
@Composable
private fun IdleContent(onChooseFile: () -> Unit) {
    ImportExplanation()
    PrimaryChooseFile(onChooseFile)
}

/** The file summary, the name field, the size refusal, the failure, and Import. */
@Composable
private fun PickedContent(
    state: ImportProjectUiState,
    onChooseFile: () -> Unit,
    onNameChange: (String) -> Unit,
    onImport: () -> Unit,
) {
    FileSummary(state)
    if (state.tooLarge) {
        val size = formatArchiveSize(state.sizeBytes ?: 0L)
        InlineBanner(
            text = stringResource(R.string.archive_too_large, sizeText(size)),
            tone = BannerTone.Danger,
        )
    }
    ProjectNameField(state = state, onNameChange = onNameChange)
    state.reason?.let { reason ->
        InlineBanner(
            text = stringResource(reason.messageRes()),
            tone = BannerTone.Danger,
            actionLabel = stringResource(R.string.archive_try_again),
            onAction = onImport,
        )
    }
    PrimaryImport(state, onImport)
    ChangeFile(onChooseFile)
}

/** The indeterminate progress and Cancel while the core unpacks the ZIP. */
@Composable
private fun ImportingContent(onCancel: () -> Unit) {
    ProgressWithText(text = stringResource(R.string.archive_importing))
    ImportCancel(onCancel)
}

/** A short line while the screen closes and the project opens. */
@Composable
private fun DoneContent() {
    ProgressWithText(text = stringResource(R.string.archive_import_done))
}
