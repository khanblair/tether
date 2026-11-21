package app.tether.memory

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * Writes a note: the whole file, or text added to the end of an existing note. The path rules show
 * under the path field. [onSaved] gets the path once the core has written the note.
 */
@Composable
fun VaultEditScreen(
    viewModel: VaultEditViewModel,
    onSaved: (String) -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LaunchedEffect(state.savedPath) { state.savedPath?.let(onSaved) }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title =
                    stringResource(
                        if (state.existing) R.string.vault_edit_title else R.string.vault_new_title,
                    ),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxWidth()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Column(
                modifier = Modifier.widthIn(max = TetherSize.transcriptMax).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
            ) {
                EditorContent(state = state, viewModel = viewModel)
            }
        }
    }
}

@Composable
private fun EditorContent(
    state: VaultEditUiState,
    viewModel: VaultEditViewModel,
) {
    state.problem?.let {
        InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
    }
    if (state.loading) return
    AreaTextField(
        value = state.path,
        onValueChange = viewModel::onPathChange,
        label = stringResource(R.string.vault_path_label),
        hint = stringResource(R.string.vault_path_hint),
        error = state.pathProblem?.let { stringResource(it.messageRes()) },
        mono = true,
        enabled = !state.existing,
    )
    if (state.existing) {
        VaultModeTabs(mode = state.mode, onSelect = viewModel::onModeChange)
    }
    AreaTextField(
        value = state.source,
        onValueChange = viewModel::onSourceChange,
        label = stringResource(sourceLabel(state.mode)),
        hint = stringResource(R.string.vault_source_hint),
        mono = true,
        minLines = SOURCE_LINES,
    )
    PrimaryButton(
        label = stringResource(R.string.vault_save),
        onClick = viewModel::save,
        enabled = state.source.isNotBlank() && !state.saving,
        loading = state.saving,
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun VaultModeTabs(
    mode: VaultEditMode,
    onSelect: (VaultEditMode) -> Unit,
) {
    val modes = VaultEditMode.entries
    AreaTabs(
        labels = modes.map { stringResource(modeLabel(it)) },
        selected = modes.indexOf(mode),
        onSelect = { onSelect(modes[it]) },
    )
}

private fun modeLabel(mode: VaultEditMode): Int =
    when (mode) {
        VaultEditMode.Replace -> R.string.vault_mode_replace
        VaultEditMode.Append -> R.string.vault_mode_append
    }

private fun sourceLabel(mode: VaultEditMode): Int =
    when (mode) {
        VaultEditMode.Replace -> R.string.vault_source_label
        VaultEditMode.Append -> R.string.vault_append_label
    }

private const val SOURCE_LINES = 12
