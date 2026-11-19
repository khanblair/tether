package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.PrimaryButton
import app.tether.design.components.ProjectRow
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * The Projects screen, a drawer destination. The top bar opens the drawer and searches projects
 * by name. The bottom action starts the New project sheet.
 */
@Composable
fun ProjectsScreen(
    viewModel: ProjectsViewModel,
    onOpenMenu: () -> Unit,
    onOpenProject: (String) -> Unit,
    onNewProject: (NewProjectChoice) -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    var searching by rememberSaveable { mutableStateOf(false) }
    var sheetOpen by rememberSaveable { mutableStateOf(false) }
    val hasProjects = state.totalCount > 0
    val toggleSearch = {
        if (searching) {
            searching = false
            viewModel.onQueryChange("")
        } else {
            searching = true
        }
    }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            ProjectsTopBar(
                showSearch = hasProjects,
                onOpenMenu = onOpenMenu,
                onToggleSearch = toggleSearch,
            )
        },
        bottomBar = {
            NewProjectBar(onClick = { sheetOpen = true })
        },
    ) { padding ->
        ProjectsBody(
            state = state,
            searching = searching,
            onQueryChange = viewModel::onQueryChange,
            onOpenProject = onOpenProject,
            modifier = Modifier.padding(padding),
        )
    }
    if (sheetOpen) {
        NewProjectSheet(
            onDismiss = { sheetOpen = false },
            onChoose = { choice ->
                sheetOpen = false
                onNewProject(choice)
            },
        )
    }
}

@Composable
private fun ProjectsTopBar(
    showSearch: Boolean,
    onOpenMenu: () -> Unit,
    onToggleSearch: () -> Unit,
) {
    TetherTopBar(
        title = stringResource(R.string.route_projects_title),
        navigationIcon = {
            TetherIconButton(
                icon = TetherIcons.Menu,
                contentDescription = stringResource(R.string.home_menu_action),
                onClick = onOpenMenu,
            )
        },
        actions = {
            if (showSearch) {
                TetherIconButton(
                    icon = TetherIcons.Search,
                    contentDescription = stringResource(R.string.home_search_action),
                    onClick = onToggleSearch,
                )
            }
        },
    )
}

@Composable
private fun NewProjectBar(onClick: () -> Unit) {
    Box(
        modifier =
            Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.navigationBars)
                .padding(TetherSpace.space4),
    ) {
        PrimaryButton(
            label = stringResource(R.string.home_new_project),
            onClick = onClick,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}

@Composable
private fun ProjectsBody(
    state: ProjectsUiState,
    searching: Boolean,
    onQueryChange: (String) -> Unit,
    onOpenProject: (String) -> Unit,
    modifier: Modifier,
) {
    if (state.totalCount == 0) {
        Text(
            text = stringResource(R.string.home_empty_body),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
            modifier = modifier.padding(TetherSpace.space4),
        )
        return
    }
    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        if (searching) {
            item {
                TetherSearchField(
                    value = state.query,
                    onValueChange = onQueryChange,
                    placeholder = stringResource(R.string.home_search_placeholder),
                    clearLabel = stringResource(R.string.home_search_clear),
                )
            }
        }
        item {
            if (state.projects.isEmpty()) {
                NoMatch(query = state.query)
            } else {
                ProjectList(state = state, onOpenProject = onOpenProject)
            }
        }
        item {
            Text(
                text =
                    pluralStringResource(
                        R.plurals.home_project_count,
                        state.totalCount,
                        state.totalCount,
                    ),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
                modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space4),
            )
        }
    }
}

@Composable
private fun NoMatch(query: String) {
    Text(
        text = stringResource(R.string.home_no_match, query.trim()),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
    )
}

@Composable
private fun ProjectList(
    state: ProjectsUiState,
    onOpenProject: (String) -> Unit,
) {
    ListGroup {
        state.projects.forEachIndexed { index, project ->
            if (index > 0) {
                HorizontalDivider(
                    thickness = TetherBorder.hairline,
                    color = TetherTheme.colors.line,
                )
            }
            val runningBadge: @Composable () -> Unit = { RunningProjectBadge() }
            ProjectRow(
                name = project.name,
                branch = project.branch,
                summary = projectSummary(project, state.now),
                onClick = { onOpenProject(project.id) },
                badge = if (project.running) runningBadge else null,
            )
        }
    }
}
