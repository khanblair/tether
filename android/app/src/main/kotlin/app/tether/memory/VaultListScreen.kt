package app.tether.memory

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/** The vault's Notes tab: search, the standard notes, and New note. */
@Composable
fun VaultListScreen(
    viewModel: VaultListViewModel,
    onOpen: (String) -> Unit,
    onNew: () -> Unit,
    onOpenTab: (VaultTab) -> Unit,
    onSync: () -> Unit,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    LifecycleResumeEffect(Unit) {
        viewModel.refresh()
        onPauseOrDispose {}
    }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.vault_title),
                navigationIcon = { BackButton(onBack) },
                actions = {
                    TetherTextButton(
                        label = stringResource(R.string.vault_sync),
                        onClick = onSync,
                    )
                },
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
                VaultTabs(selected = VaultTab.Notes, onSelect = onOpenTab)
                VaultListContent(
                    state = state,
                    onQueryChange = viewModel::onQueryChange,
                    onOpen = onOpen,
                )
                PrimaryButton(
                    label = stringResource(R.string.vault_new_note),
                    onClick = onNew,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

@Composable
private fun VaultListContent(
    state: VaultListUiState,
    onQueryChange: (String) -> Unit,
    onOpen: (String) -> Unit,
) {
    TetherSearchField(
        value = state.query,
        onValueChange = onQueryChange,
        placeholder = stringResource(R.string.vault_search_placeholder),
        clearLabel = stringResource(R.string.vault_search_clear),
    )
    state.problem?.let {
        InlineBanner(text = stringResource(it.messageRes()), tone = BannerTone.Danger)
    }
    if (state.loading) return
    when {
        state.notes.isNotEmpty() -> NoteGroup(notes = state.notes, onOpen = onOpen)
        state.query.isBlank() -> Hint(R.string.vault_standard_hint)
        else -> Hint(R.string.vault_no_hits)
    }
}

@Composable
private fun Hint(textRes: Int) {
    Text(
        text = stringResource(textRes),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text2,
    )
}

@Composable
private fun NoteGroup(
    notes: List<VaultNote>,
    onOpen: (String) -> Unit,
) {
    val shape = RoundedCornerShape(TetherRadius.md)
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .clip(shape)
                .background(TetherTheme.colors.surface)
                .border(TetherBorder.hairline, TetherTheme.colors.line, shape),
    ) {
        notes.forEachIndexed { index, note ->
            if (index > 0) {
                HorizontalDivider(
                    color = TetherTheme.colors.line,
                    thickness = TetherBorder.hairline,
                )
            }
            ListRow(
                title = note.title,
                subtitle = note.description,
                onClick = { onOpen(note.path) },
            )
        }
    }
}
