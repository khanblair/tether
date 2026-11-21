package app.tether.files

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * The file browser for one project. Folders expand on tap and load once. A search field shows
 * file name matches as the user types, and the text search runs when its row is tapped. Files open
 * in the viewer. The screen only reads: changes go through the agent.
 */
@Composable
fun FilesScreen(
    viewModel: FilesViewModel,
    onBack: () -> Unit,
    onOpenFile: (String) -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    // Expanded folders survive rotation and process death. The cache reloads them on restore.
    val expanded = rememberSaveable { mutableStateListOf<String>() }
    LaunchedEffect(Unit) { expanded.forEach(viewModel::ensureLoaded) }
    val expandedSet = expanded.toSet()
    val rows = remember(state.cache, expandedSet) { visibleRows(state.cache, expandedSet) }
    val toggle: (String) -> Unit = { path ->
        if (path in expanded) {
            expanded.remove(path)
        } else {
            expanded.add(path)
            viewModel.ensureLoaded(path)
        }
    }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            FilesTopBar(
                projectName = state.projectName,
                showIgnored = state.showIgnored,
                onBack = onBack,
                onToggleSearch = { viewModel.setSearchOpen(!state.searchOpen) },
                onShowIgnored = { viewModel.setShowIgnored(it, expandedSet) },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.fillMaxSize().padding(padding)) {
            if (state.searchOpen) {
                TetherSearchField(
                    value = state.query,
                    onValueChange = viewModel::onQueryChange,
                    placeholder = stringResource(R.string.files_search_placeholder),
                    clearLabel = stringResource(R.string.files_search_clear),
                    modifier =
                        Modifier.padding(
                            horizontal = TetherSpace.space4,
                            vertical = TetherSpace.space2,
                        ),
                )
            }
            if (state.searchOpen && state.query.isNotBlank()) {
                SearchResults(
                    state = state,
                    onSearchText = viewModel::searchText,
                    onOpenFile = onOpenFile,
                    modifier = Modifier.fillMaxWidth(),
                )
            } else {
                TreeList(
                    rows = rows,
                    onToggle = toggle,
                    onOpenFile = onOpenFile,
                    onRetry = viewModel::retry,
                )
            }
        }
    }
}

@Composable
private fun FilesTopBar(
    projectName: String?,
    showIgnored: Boolean,
    onBack: () -> Unit,
    onToggleSearch: () -> Unit,
    onShowIgnored: (Boolean) -> Unit,
) {
    var menuOpen by rememberSaveable { mutableStateOf(false) }
    TetherTopBar(
        title = stringResource(R.string.route_files_title),
        subtitle = projectName,
        navigationIcon = { BackButton(onBack) },
        actions = {
            TetherIconButton(
                icon = TetherIcons.Search,
                contentDescription = stringResource(R.string.files_search_action),
                onClick = onToggleSearch,
            )
            Box {
                TetherTextButton(
                    label = stringResource(R.string.files_more_action),
                    onClick = { menuOpen = true },
                )
                ShowIgnoredMenu(
                    open = menuOpen,
                    showIgnored = showIgnored,
                    onDismiss = { menuOpen = false },
                    onToggle = {
                        menuOpen = false
                        onShowIgnored(!showIgnored)
                    },
                )
            }
        },
    )
}

/** The overflow menu. Its one item is the ignored toggle, which says On or Off to TalkBack. */
@Composable
private fun ShowIgnoredMenu(
    open: Boolean,
    showIgnored: Boolean,
    onDismiss: () -> Unit,
    onToggle: () -> Unit,
) {
    val onLabel = stringResource(R.string.files_state_on)
    val offLabel = stringResource(R.string.files_state_off)
    DropdownMenu(expanded = open, onDismissRequest = onDismiss) {
        DropdownMenuItem(
            text = { Text(stringResource(R.string.files_show_ignored)) },
            onClick = onToggle,
            leadingIcon = {
                TetherIcon(
                    icon = if (showIgnored) TetherIcons.CircleCheck else TetherIcons.CircleDot,
                    contentDescription = null,
                    tint = TetherTheme.colors.text2,
                )
            },
            modifier =
                Modifier.semantics {
                    stateDescription = if (showIgnored) onLabel else offLabel
                },
        )
    }
}

@Composable
private fun TreeList(
    rows: List<TreeRow>,
    onToggle: (String) -> Unit,
    onOpenFile: (String) -> Unit,
    onRetry: (String) -> Unit,
) {
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(vertical = TetherSpace.space2),
    ) {
        items(count = rows.size, key = { rowKey(rows[it], it) }) { index ->
            FileTreeRowItem(
                row = rows[index],
                onToggle = onToggle,
                onOpenFile = onOpenFile,
                onRetry = onRetry,
            )
        }
    }
}

/** Entries key on their path so that expanding a folder keeps the rows below it in place. */
private fun rowKey(
    row: TreeRow,
    index: Int,
): String = if (row is TreeRow.Entry) "entry:${row.entry.path}" else "row:$index"

/** The name matches, then the Search text row and its lines. */
@Composable
private fun SearchResults(
    state: FilesUiState,
    onSearchText: () -> Unit,
    onOpenFile: (String) -> Unit,
    modifier: Modifier,
) {
    LazyColumn(
        modifier = modifier,
        contentPadding =
            PaddingValues(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        item {
            ListGroup {
                ListRow(
                    title = stringResource(R.string.files_text_title),
                    subtitle = stringResource(R.string.files_text_body, state.query.trim()),
                    onClick = onSearchText,
                )
            }
        }
        if (state.nameProblem) {
            item { NameProblem() }
        }
        val names = state.nameHits
        if (names != null && names.isEmpty()) {
            item { NoMatches(R.string.files_nothing_matches) }
        } else if (names != null) {
            item { PathGroup(names, onOpenFile = onOpenFile) }
        }
        textSearchItems(state.textSearch, onOpenFile)
    }
}

private fun LazyListScope.textSearchItems(
    search: TextSearch,
    onOpenFile: (String) -> Unit,
) {
    when (search) {
        TextSearch.Idle -> Unit
        TextSearch.Loading -> item { ThinLoadingLine(stringResource(R.string.files_loading)) }
        TextSearch.Failed -> item { TextProblem() }
        is TextSearch.Hits ->
            if (search.hits.isEmpty()) {
                item { NoMatches(R.string.files_text_none) }
            } else {
                item { HitGroup(search.hits, onOpenFile) }
            }
    }
}

@Composable
private fun NameProblem() {
    InlineBanner(
        text = stringResource(R.string.files_name_search_failed),
        tone = BannerTone.Danger,
    )
}

@Composable
private fun TextProblem() {
    InlineBanner(
        text = stringResource(R.string.files_text_failed),
        tone = BannerTone.Danger,
    )
}

@Composable
private fun NoMatches(textRes: Int) {
    Text(
        text = stringResource(textRes),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text2,
    )
}

@Composable
private fun PathGroup(
    paths: List<String>,
    onOpenFile: (String) -> Unit,
) {
    ListGroup {
        paths.forEachIndexed { index, path ->
            if (index > 0) FileDivider()
            ListRow(title = path, onClick = { onOpenFile(path) })
        }
    }
}

@Composable
private fun HitGroup(
    hits: List<FileTextHit>,
    onOpenFile: (String) -> Unit,
) {
    ListGroup {
        hits.forEachIndexed { index, hit ->
            if (index > 0) FileDivider()
            ListRow(
                title = hit.path,
                subtitle = stringResource(R.string.files_text_hit, hit.line, hit.text.trim()),
                onClick = { onOpenFile(hit.path) },
            )
        }
    }
}
