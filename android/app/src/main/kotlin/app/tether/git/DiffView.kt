package app.tether.git

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.tokens.TetherSpace

/**
 * The diff as one lazy list of [rows]: a file header, its hunks, and their lines. Wrapping follows
 * [wrap]. [listState] is owned by the caller, so chip taps can scroll to a file.
 */
@Composable
fun DiffView(
    rows: List<DiffRow>,
    wrap: Boolean,
    listState: LazyListState,
    modifier: Modifier = Modifier,
) {
    LazyColumn(
        state = listState,
        modifier = modifier.fillMaxWidth(),
        contentPadding = PaddingValues(bottom = TetherSpace.space6),
    ) {
        items(rows) { row ->
            DiffRowItem(row = row, wrap = wrap)
        }
    }
}

@Composable
private fun DiffRowItem(
    row: DiffRow,
    wrap: Boolean,
) {
    when (row) {
        is DiffRow.FileHeader -> FileHeaderRow(row.file)
        is DiffRow.Binary -> MutedRow(stringResource(R.string.git_diff_binary))
        is DiffRow.NoText -> MutedRow(stringResource(R.string.git_diff_no_text))
        is DiffRow.HunkHeader -> HunkHeaderRow(row.header)
        is DiffRow.Line -> DiffLine(line = row.line, wrap = wrap)
    }
}

/** A file's path with its added and removed counts. */
@Composable
private fun FileHeaderRow(file: DiffFile) {
    val colors = TetherTheme.colors
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .background(colors.surfaceSunken)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        Text(
            text = file.path,
            style = TetherTheme.monoLabel,
            color = colors.text,
            maxLines = 1,
            overflow = TextOverflow.MiddleEllipsis,
            modifier = Modifier.weight(1f),
        )
        FileCounts(added = file.added, removed = file.removed)
    }
}

/** The `+n` and `−m` counts in their status colors. */
@Composable
internal fun FileCounts(
    added: Int,
    removed: Int,
) {
    val colors = TetherTheme.colors
    Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space1)) {
        Text(
            text = stringResource(R.string.git_diff_added_count, added),
            style = MaterialTheme.typography.labelMedium,
            color = colors.success,
        )
        Text(
            text = stringResource(R.string.git_diff_removed_count, removed),
            style = MaterialTheme.typography.labelMedium,
            color = colors.danger,
        )
    }
}

@Composable
private fun HunkHeaderRow(header: String) {
    Text(
        text = header,
        style = TetherTheme.monoSmall,
        color = TetherTheme.colors.text3,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
        modifier =
            Modifier
                .fillMaxWidth()
                .background(TetherTheme.colors.codeBg)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space1),
    )
}

@Composable
private fun MutedRow(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text3,
        modifier =
            Modifier.fillMaxWidth().padding(
                horizontal = TetherSpace.space4,
                vertical = TetherSpace.space2,
            ),
    )
}
