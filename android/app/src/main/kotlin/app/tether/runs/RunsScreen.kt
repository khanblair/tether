package app.tether.runs

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Scaffold
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ChipTone
import app.tether.design.components.TetherChip
import app.tether.design.components.TetherIconButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.settings.executors.CiRunsContent
import app.tether.settings.executors.text
import app.tether.shell.BackButton

/** The full project run history, with local and GitHub rows merged by start time. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RunsScreen(
    viewModel: RunsViewModel,
    projectName: String?,
    onBack: () -> Unit,
    onOpenProjectPicker: () -> Unit,
    onOpenLocal: (String) -> Unit,
    onOpenCi: (String) -> Unit,
    onRestart: (RunRow.Local) -> Unit,
) {
    val state = viewModel.state.collectAsStateWithLifecycle().value
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.runs_title),
                subtitle = projectName,
                navigationIcon = { BackButton(onBack) },
                actions = {
                    TetherIconButton(
                        icon = TetherIcons.Folder,
                        contentDescription = stringResource(R.string.runs_choose_project),
                        onClick = onOpenProjectPicker,
                    )
                },
            )
        },
    ) { padding ->
        RunsBody(
            state = state,
            viewModel = viewModel,
            projectName = projectName,
            onOpenProjectPicker = onOpenProjectPicker,
            onOpenLocal = onOpenLocal,
            onOpenCi = onOpenCi,
            onRestart = onRestart,
            modifier = Modifier.padding(padding),
        )
    }
}

@Composable
private fun RunsBody(
    state: RunsUiState,
    viewModel: RunsViewModel,
    projectName: String?,
    onOpenProjectPicker: () -> Unit,
    onOpenLocal: (String) -> Unit,
    onOpenCi: (String) -> Unit,
    onRestart: (RunRow.Local) -> Unit,
    modifier: Modifier,
) {
    val ciFailure = state.ciContent as? CiRunsContent.Failed
    val failureText =
        if (state.filter == RunsFilter.Ci) {
            ciFailure?.failure?.text()
        } else {
            null
        }
    Column(
        modifier = modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        RunsFilterChips(
            selected = state.filter,
            onSelect = viewModel::setFilter,
            projectName = projectName,
            onOpenProjectPicker = onOpenProjectPicker,
        )
        PullToRefreshBox(
            isRefreshing = state.refreshing,
            onRefresh = viewModel::refresh,
            modifier = Modifier.weight(1f).fillMaxWidth(),
        ) {
            RunsList(
                content = state.content,
                onRetry = viewModel::refresh,
                onOpenLocal = onOpenLocal,
                onOpenCi = onOpenCi,
                onStop = viewModel::stop,
                onRestart = onRestart,
                modifier = Modifier.fillMaxSize(),
                ciContent = state.ciContent.takeIf { state.filter == RunsFilter.Ci },
                failureText = failureText,
            )
        }
    }
}

/** The project chip, then the four executor filters from the prototype. */
@Composable
fun RunsFilterChips(
    selected: RunsFilter,
    onSelect: (RunsFilter) -> Unit,
    projectName: String?,
    onOpenProjectPicker: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier =
            modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState())
                .padding(horizontal = TetherSpace.space4),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        projectName?.let { name ->
            TetherChip(
                label = name,
                trailingChevron = true,
                onClick = onOpenProjectPicker,
            )
        }
        RunsFilter.entries.forEach { filter ->
            TetherChip(
                label = stringResource(filter.labelRes()),
                tone = if (filter == selected) ChipTone.Accent else ChipTone.Neutral,
                onClick = { onSelect(filter) },
            )
        }
    }
}

private fun RunsFilter.labelRes(): Int =
    when (this) {
        RunsFilter.All -> R.string.runs_filter_all
        RunsFilter.Sandbox -> R.string.runs_filter_sandbox
        RunsFilter.Linux -> R.string.runs_filter_linux
        RunsFilter.Ci -> R.string.runs_filter_ci
    }
