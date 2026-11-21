package app.tether.memory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** Shows the agent's two memory files. Its state comes from [viewModel]. */
@Composable
fun MemoryScreen(
    viewModel: MemoryViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.memory_title),
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
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
            ) {
                FileTabs(selected = state.file, onSelect = viewModel::select)
                MemoryMeter(used = state.used, cap = state.cap)
                MemoryBody(state = state, viewModel = viewModel)
            }
        }
    }
}

@Composable
private fun MemoryBody(
    state: MemoryUiState,
    viewModel: MemoryViewModel,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space3)) {
        Text(
            text = stringResource(R.string.memory_read_note),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
        if (state.readFailed) {
            InlineBanner(
                text = stringResource(R.string.memory_read_failed),
                tone = BannerTone.Danger,
            )
        }
        state.problem?.let {
            InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
        }
        if (!state.loading && !state.readFailed) {
            EntryList(state = state, viewModel = viewModel)
            AddEntry(state = state, viewModel = viewModel)
        }
    }
}

/** The file choice: one tab per memory file. */
@Composable
private fun FileTabs(
    selected: MemoryFile,
    onSelect: (MemoryFile) -> Unit,
) {
    val files = MemoryFile.entries
    AreaTabs(
        labels = files.map { stringResource(it.labelRes()) },
        selected = files.indexOf(selected),
        onSelect = { onSelect(files[it]) },
    )
}

private fun MemoryFile.labelRes(): Int =
    when (this) {
        MemoryFile.Memory -> R.string.memory_file_memory
        MemoryFile.User -> R.string.memory_file_user
    }

/** The character count as plain text, with a thin meter under it. */
@Composable
private fun MemoryMeter(
    used: Int,
    cap: Int,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        Text(
            text = stringResource(R.string.memory_cap_format, groupDigits(used), groupDigits(cap)),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
        LinearProgressIndicator(
            progress = { capFraction(used, cap) },
            modifier = Modifier.fillMaxWidth(),
            color = TetherTheme.colors.text2,
            trackColor = TetherTheme.colors.line,
        )
    }
}

@Composable
private fun EntryList(
    state: MemoryUiState,
    viewModel: MemoryViewModel,
) {
    if (state.entries.isEmpty()) {
        Text(
            text = stringResource(R.string.memory_empty),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
        )
        return
    }
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        state.entries.forEachIndexed { index, entry ->
            if (index > 0) {
                HorizontalDivider(
                    color = TetherTheme.colors.line,
                    thickness = TetherBorder.hairline,
                )
            }
            EntryRow(index = index, entry = entry, state = state, viewModel = viewModel)
        }
    }
}

@Composable
private fun EntryRow(
    index: Int,
    entry: String,
    state: MemoryUiState,
    viewModel: MemoryViewModel,
) {
    Column(modifier = Modifier.fillMaxWidth().padding(TetherSpace.space4)) {
        if (state.editing == index) {
            EditEntry(state = state, viewModel = viewModel)
        } else {
            Text(
                text = entry,
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text,
            )
            Row(horizontalArrangement = Arrangement.End, modifier = Modifier.fillMaxWidth()) {
                TetherTextButton(
                    label = stringResource(R.string.memory_entry_edit),
                    onClick = { viewModel.startEdit(index) },
                )
                TetherTextButton(
                    label = stringResource(R.string.memory_entry_remove),
                    onClick = { viewModel.remove(index) },
                    tone = TextButtonTone.Danger,
                )
            }
        }
    }
}

@Composable
private fun EditEntry(
    state: MemoryUiState,
    viewModel: MemoryViewModel,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        AreaTextField(
            value = state.editText,
            onValueChange = viewModel::onEditTextChange,
            label = stringResource(R.string.memory_edit_label),
            minLines = EDIT_LINES,
        )
        Row(horizontalArrangement = Arrangement.End, modifier = Modifier.fillMaxWidth()) {
            TetherTextButton(
                label = stringResource(R.string.memory_entry_cancel),
                onClick = viewModel::cancelEdit,
            )
            PrimaryButton(
                label = stringResource(R.string.memory_entry_save),
                onClick = viewModel::saveEdit,
                enabled = state.editText.isNotBlank(),
            )
        }
    }
}

@Composable
private fun AddEntry(
    state: MemoryUiState,
    viewModel: MemoryViewModel,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        AreaTextField(
            value = state.draft,
            onValueChange = viewModel::onDraftChange,
            label = stringResource(R.string.memory_draft_label),
            hint = stringResource(R.string.memory_draft_hint),
            minLines = EDIT_LINES,
        )
        PrimaryButton(
            label = stringResource(R.string.memory_draft_add),
            onClick = viewModel::add,
            enabled = state.draft.isNotBlank(),
            modifier = Modifier.align(Alignment.End),
        )
    }
}

private const val EDIT_LINES = 2
