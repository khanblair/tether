package app.tether.git

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
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
import kotlinx.coroutines.launch

/**
 * The diff screen for one file, or for all changes when [DiffUiState.path] is null. It has the
 * Staged and Not staged switch, the file chips, and the diff. [onSetStaged] reads the other side,
 * [onToggleWrap] turns line wrapping on or off, and [onRetry] reads the diff again.
 */
@Composable
fun DiffScreen(
    state: DiffUiState,
    onSetStaged: (Boolean) -> Unit,
    onToggleWrap: () -> Unit,
    onRetry: () -> Unit,
    onBack: () -> Unit,
) {
    val allTitle = stringResource(R.string.git_diff_title_all)
    val stagedLabel = stringResource(R.string.git_section_staged)
    val unstagedLabel = stringResource(R.string.git_section_unstaged)
    val wrapLabel =
        stringResource(if (state.wrap) R.string.git_diff_wrap_off else R.string.git_diff_wrap_on)
    val listState = rememberLazyListState()
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = state.path ?: allTitle,
                navigationIcon = { BackButton(onBack) },
                actions = {
                    TetherTextButton(label = wrapLabel, onClick = onToggleWrap)
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.padding(padding).fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            SegmentedControl(
                options = listOf(true, false),
                selected = state.staged,
                onSelect = onSetStaged,
                label = { if (it) stagedLabel else unstagedLabel },
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
            LoadableContent(state.content, onRetry) { content ->
                DiffBody(
                    content = content,
                    path = state.path,
                    wrap = state.wrap,
                    listState = listState,
                )
            }
        }
    }
}

/**
 * The loaded diff: a banner when the core cut it, then the file chips and the list. An empty
 * diff gets one sentence instead. A chip tap scrolls the list to that file's header.
 */
@Composable
private fun DiffBody(
    content: DiffContent,
    path: String?,
    wrap: Boolean,
    listState: LazyListState,
) {
    val rows = remember(content.files) { flattenDiff(content.files) }
    val scope = rememberCoroutineScope()
    val reducedMotion = rememberReducedMotion()
    Column(modifier = Modifier.fillMaxSize()) {
        if (content.truncated) {
            InlineBanner(
                text = stringResource(R.string.git_diff_truncated),
                tone = BannerTone.Warning,
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = TetherSpace.space4),
            )
        }
        if (content.files.isEmpty()) {
            EmptyDiff(path)
        } else {
            DiffFileChips(
                files = content.files,
                onPick = { fileIndex ->
                    scope.launch { scrollToFile(listState, rows, fileIndex, reducedMotion) }
                },
            )
            DiffView(
                rows = rows,
                wrap = wrap,
                listState = listState,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

/** Scrolls the list to file [fileIndex]'s header. Reduced motion jumps instead. */
private suspend fun scrollToFile(
    listState: LazyListState,
    rows: List<DiffRow>,
    fileIndex: Int,
    reducedMotion: Boolean,
) {
    val row = rowIndexOfFile(rows, fileIndex)
    if (row < 0) return
    if (reducedMotion) {
        listState.scrollToItem(row)
    } else {
        listState.animateScrollToItem(row)
    }
}

@Composable
private fun EmptyDiff(path: String?) {
    Box(
        modifier = Modifier.fillMaxSize().padding(TetherSpace.space4),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text =
                stringResource(
                    if (path == null) R.string.git_diff_empty_all else R.string.git_diff_empty_file,
                ),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
            textAlign = TextAlign.Center,
        )
    }
}
