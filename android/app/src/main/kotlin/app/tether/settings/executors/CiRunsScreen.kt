package app.tether.settings.executors

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * The last [CI_RUN_LIMIT] workflow runs of one project. Pull down to read them again. A project
 * that cannot run CI yet shows the reason instead of a list.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CiRunsScreen(
    viewModel: CiRunsViewModel,
    onBack: () -> Unit,
    onOpenUrl: (String) -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.executors_runs_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        PullToRefreshBox(
            isRefreshing = state.refreshing,
            onRefresh = viewModel::refresh,
            modifier = Modifier.padding(padding).fillMaxSize(),
        ) {
            when (val content = state.content) {
                CiRunsContent.Loading -> CiRunsList(runs = null, onOpen = onOpenUrl)
                is CiRunsContent.Runs -> CiRunsList(runs = content.rows, onOpen = onOpenUrl)
                CiRunsContent.Empty -> CiRunsList(runs = emptyList(), onOpen = onOpenUrl)
                else -> ScrollableMessage(content = content, onRetry = viewModel::refresh)
            }
        }
    }
}

/**
 * The text for the states that have no list. The column scrolls, so a pull to refresh still
 * works on it.
 */
@Composable
private fun ScrollableMessage(
    content: CiRunsContent,
    onRetry: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(TetherSpace.space4),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        ListGroup {
            Column(modifier = Modifier.padding(TetherSpace.space4)) {
                Text(
                    text = messageText(content),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text2,
                )
                if (content is CiRunsContent.Failed) {
                    TetherTextButton(
                        label = stringResource(R.string.executors_try_again),
                        onClick = onRetry,
                    )
                }
            }
        }
    }
}

/** The sentence for a state without a list. */
@Composable
private fun messageText(content: CiRunsContent): String =
    when (content) {
        CiRunsContent.NoProject -> stringResource(R.string.executors_runs_no_project)
        is CiRunsContent.NotReady -> content.readiness.text()
        is CiRunsContent.Failed -> content.failure.text()
        CiRunsContent.Loading, CiRunsContent.Empty, is CiRunsContent.Runs -> ""
    }
