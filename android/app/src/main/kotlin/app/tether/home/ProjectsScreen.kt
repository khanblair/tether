package app.tether.home

import androidx.annotation.StringRes
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
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BadgeTone
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.NotificationBell
import app.tether.design.components.PrimaryButton
import app.tether.design.components.ProjectRow
import app.tether.design.components.StatusBadge
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * The Projects screen, the app's home. The top bar opens the drawer, searches projects by name,
 * and opens Needs attention. The bottom action starts the New project sheet.
 */
@Composable
fun ProjectsScreen(
    viewModel: ProjectsViewModel,
    hasAttention: Boolean,
    attentionCount: Int,
    onOpenMenu: () -> Unit,
    onOpenAttention: () -> Unit,
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
                hasAttention = hasAttention,
                attentionCount = attentionCount,
                onOpenMenu = onOpenMenu,
                onToggleSearch = toggleSearch,
                onOpenAttention = onOpenAttention,
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
    hasAttention: Boolean,
    attentionCount: Int,
    onOpenMenu: () -> Unit,
    onToggleSearch: () -> Unit,
    onOpenAttention: () -> Unit,
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
            NotificationBell(
                hasAttention = hasAttention,
                contentDescription = attentionLabel(attentionCount),
                onClick = onOpenAttention,
            )
        },
    )
}

@Composable
private fun attentionLabel(count: Int): String =
    if (count > 0) {
        pluralStringResource(R.plurals.home_bell_count, count, count)
    } else {
        stringResource(R.string.home_bell_action)
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
            val ago = relativeTime(state.now, project.lastActivityAt).text()
            val summary =
                if (project.uncommitted > 0) {
                    pluralStringResource(
                        R.plurals.home_project_dirty,
                        project.uncommitted,
                        project.uncommitted,
                        ago,
                    )
                } else {
                    stringResource(R.string.home_project_clean, ago)
                }
            val runningBadge: @Composable () -> Unit = {
                StatusBadge(
                    label = stringResource(R.string.home_running_badge),
                    tone = BadgeTone.Tonal,
                    icon = TetherIcons.Loader,
                )
            }
            ProjectRow(
                name = project.name,
                branch = project.branch,
                summary = summary,
                onClick = { onOpenProject(project.id) },
                badge = if (project.running) runningBadge else null,
            )
        }
    }
}

@Composable
private fun NewProjectSheet(
    onDismiss: () -> Unit,
    onChoose: (NewProjectChoice) -> Unit,
) {
    TetherSheet(title = stringResource(R.string.home_new_project), onDismiss = onDismiss) {
        ListGroup(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
            SheetRow(
                icon = TetherIcons.GitBranch,
                title = R.string.home_new_project_clone,
                subtitle = R.string.home_new_project_clone_sub,
                onClick = { onChoose(NewProjectChoice.Clone) },
            )
            HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
            SheetRow(
                icon = TetherIcons.FolderOpen,
                title = R.string.home_new_project_empty,
                subtitle = R.string.home_new_project_empty_sub,
                onClick = { onChoose(NewProjectChoice.Empty) },
            )
            HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
            SheetRow(
                icon = TetherIcons.Package,
                title = R.string.home_new_project_import,
                subtitle = R.string.home_new_project_import_sub,
                onClick = { onChoose(NewProjectChoice.Import) },
            )
        }
    }
}

@Composable
private fun SheetRow(
    icon: ImageVector,
    @StringRes title: Int,
    @StringRes subtitle: Int,
    onClick: () -> Unit,
) {
    ListRow(
        title = stringResource(title),
        subtitle = stringResource(subtitle),
        leading = {
            TetherIcon(icon = icon, contentDescription = null, tint = TetherTheme.colors.text2)
        },
        onClick = onClick,
    )
}
