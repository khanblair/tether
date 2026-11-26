package app.tether.home

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
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
import androidx.lifecycle.compose.LifecycleResumeEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.NotificationBell
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace

/**
 * Home, the start screen: a summary sentence and cards for what needs the user, today's usage,
 * the seven-day usage, recent projects, and the saved provider keys. The keys and the sources
 * reload each time Home shows.
 */
@Composable
fun DashboardScreen(
    viewModel: DashboardViewModel,
    attentionCount: Int,
    onOpenMenu: () -> Unit,
    onOpenAttention: () -> Unit,
    onOpenSession: (String) -> Unit,
    onOpenAbout: () -> Unit,
    onOpenProject: (String) -> Unit,
    onOpenProjects: () -> Unit,
    onOpenProviders: () -> Unit,
    onNewProject: (NewProjectChoice) -> Unit,
    onRenameSession: (SessionSummary) -> Unit = {},
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    var sheetOpen by rememberSaveable { mutableStateOf(false) }
    LifecycleResumeEffect(Unit) {
        viewModel.refresh()
        onPauseOrDispose {}
    }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            DashboardTopBar(
                attentionCount = attentionCount,
                onOpenMenu = onOpenMenu,
                onOpenAttention = onOpenAttention,
            )
        },
    ) { padding ->
        DashboardBody(
            state = state,
            onOpenAttention = onOpenAttention,
            onOpenSession = onOpenSession,
            onOpenAbout = onOpenAbout,
            onOpenProject = onOpenProject,
            onOpenProjects = onOpenProjects,
            onOpenProviders = onOpenProviders,
            onNewProjectClick = { sheetOpen = true },
            onRenameSession = onRenameSession,
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
private fun DashboardTopBar(
    attentionCount: Int,
    onOpenMenu: () -> Unit,
    onOpenAttention: () -> Unit,
) {
    TetherTopBar(
        title = stringResource(R.string.route_home_title),
        navigationIcon = {
            TetherIconButton(
                icon = TetherIcons.Menu,
                contentDescription = stringResource(R.string.home_menu_action),
                onClick = onOpenMenu,
            )
        },
        actions = {
            NotificationBell(
                hasAttention = attentionCount > 0,
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

/**
 * The cards, one column, spaced [TetherSpace.space4] apart. Each card hides when it has no data.
 */
@Composable
private fun DashboardBody(
    state: DashboardUiState,
    onOpenAttention: () -> Unit,
    onOpenSession: (String) -> Unit,
    onOpenAbout: () -> Unit,
    onOpenProject: (String) -> Unit,
    onOpenProjects: () -> Unit,
    onOpenProviders: () -> Unit,
    onNewProjectClick: () -> Unit,
    onRenameSession: (SessionSummary) -> Unit,
    modifier: Modifier,
) {
    LazyColumn(
        modifier = modifier.fillMaxSize(),
        contentPadding = PaddingValues(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space4),
    ) {
        item {
            Text(
                text = summaryText(state.summary),
                style = MaterialTheme.typography.titleMedium,
                color = TetherTheme.colors.text,
            )
        }
        if (state.attention.isNotEmpty() || state.running.isNotEmpty()) {
            item {
                NeedsCard(
                    state = state,
                    onOpenAttention = onOpenAttention,
                    onOpenSession = onOpenSession,
                    onOpenAbout = onOpenAbout,
                    onRenameSession = onRenameSession,
                )
            }
        }
        if (state.usage.hasUsage) {
            item { TodayCard(state = state) }
        }
        item { UsageCard(state = state) }
        item {
            RecentProjectsCard(
                state = state,
                onOpenProject = onOpenProject,
                onOpenProjects = onOpenProjects,
                onNewProject = onNewProjectClick,
            )
        }
        state.providers?.let { providers ->
            item {
                ProvidersCard(providers = providers, onOpenProviders = onOpenProviders)
            }
        }
    }
}
