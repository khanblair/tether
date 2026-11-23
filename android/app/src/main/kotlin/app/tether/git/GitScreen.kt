package app.tether.git

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.SegmentedControl
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * The Git screen of one project: the Changes, Branches, and History tabs. Changes lists the
 * staged and unstaged paths; tapping one opens its diff through [onOpenDiff]. Branches and History
 * are read only. [onRefresh] reads the open tab again.
 */
@Composable
fun GitScreen(
    state: GitUiState,
    onSelectTab: (GitTab) -> Unit,
    onRefresh: () -> Unit,
    onOpenDiff: (path: String?, staged: Boolean) -> Unit,
    onBack: () -> Unit,
) {
    val tabLabels = GitTab.entries.associateWith { stringResource(it.labelRes()) }
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.git_title),
                navigationIcon = { BackButton(onBack) },
                actions = {
                    TetherTextButton(
                        label = stringResource(R.string.git_refresh),
                        onClick = onRefresh,
                    )
                },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .padding(padding)
                    .fillMaxSize()
                    .padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            SegmentedControl(
                options = GitTab.entries,
                selected = state.tab,
                onSelect = onSelectTab,
                label = { tabLabels.getValue(it) },
            )
            when (state.tab) {
                GitTab.Changes ->
                    LoadableContent(state.changes, onRefresh) { changes ->
                        ChangesList(changes, onOpenDiff)
                    }
                GitTab.Branches ->
                    LoadableContent(state.branches, onRefresh) { branches ->
                        BranchesList(branches)
                    }
                GitTab.History ->
                    LoadableContent(state.history, onRefresh) { history ->
                        HistoryList(history)
                    }
            }
        }
    }
}

/**
 * Shows the spinner while a tab reads, the failure while it has failed, and [content] once the
 * data is in. Nothing shows for a tab that has not been opened yet.
 */
@Composable
internal fun <T> LoadableContent(
    state: Loadable<T>,
    onRetry: () -> Unit,
    content: @Composable (T) -> Unit,
) {
    when (state) {
        Loadable.Idle, Loadable.Loading -> GitLoading()
        is Loadable.Failed -> GitFailureBlock(state.failure, onRetry)
        is Loadable.Ready -> content(state.value)
    }
}

@Composable
private fun GitLoading() {
    val loading = stringResource(R.string.git_loading)
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        CircularProgressIndicator(
            color = TetherTheme.colors.text2,
            modifier = Modifier.semantics { contentDescription = loading },
        )
    }
}

/**
 * A plain sentence for "not a repository". Any other failure gets a danger banner with Try again.
 */
@Composable
private fun GitFailureBlock(
    failure: GitFailure,
    onRetry: () -> Unit,
) {
    val message = stringResource(failure.messageRes())
    if (failure == GitFailure.NotRepository) {
        Box(
            modifier = Modifier.fillMaxSize().padding(TetherSpace.space4),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = message,
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
                textAlign = TextAlign.Center,
            )
        }
    } else {
        InlineBanner(
            text = message,
            tone = BannerTone.Danger,
            modifier = Modifier.fillMaxWidth().padding(top = TetherSpace.space2),
            actionLabel = stringResource(R.string.git_retry),
            onAction = onRetry,
        )
    }
}
