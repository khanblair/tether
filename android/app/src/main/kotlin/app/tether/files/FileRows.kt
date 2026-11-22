package app.tether.files

import android.text.format.Formatter
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BadgeTone
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.StatusBadge
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/** The left indent of a tree row at [depth]. */
private fun indentFor(depth: Int): Dp = TetherSpace.space4 + TetherSpace.space6 * depth

/** One row of the file tree. Folders toggle, files open, and failed folders retry. */
@Composable
fun FileTreeRowItem(
    row: TreeRow,
    onToggle: (String) -> Unit,
    onOpenFile: (String) -> Unit,
    onRetry: (String) -> Unit,
) {
    val indent = indentFor(row.depth)
    val end = TetherSpace.space4
    val loadingLabel = stringResource(R.string.files_loading)
    when (row) {
        is TreeRow.Entry -> FileEntryRow(row, onToggle, onOpenFile)
        is TreeRow.Loading ->
            ThinLoadingLine(
                label = loadingLabel,
                modifier = Modifier.padding(start = indent, end = end),
            )
        is TreeRow.Failed ->
            InlineBanner(
                text = stringResource(R.string.files_folder_failed),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.files_try_again),
                onAction = { onRetry(row.dir) },
                modifier = Modifier.padding(start = indent, end = end),
            )
        is TreeRow.Empty ->
            Text(
                text = stringResource(R.string.files_empty_folder),
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text2,
                modifier = Modifier.padding(start = indent, end = end),
            )
        is TreeRow.Truncated ->
            Text(
                text = stringResource(R.string.files_truncated),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
                modifier = Modifier.padding(start = indent, end = end),
            )
    }
}

/** A file or folder row. Ignored entries are dimmed and carry the word "Ignored". */
@Composable
private fun FileEntryRow(
    row: TreeRow.Entry,
    onToggle: (String) -> Unit,
    onOpenFile: (String) -> Unit,
) {
    val entry = row.entry
    val context = LocalContext.current
    val sizeText = browserSizeText(entry) { Formatter.formatShortFileSize(context, it) }
    val rowModifier = Modifier.entryRowModifier(row, onToggle, onOpenFile)
    Row(modifier = rowModifier, verticalAlignment = Alignment.CenterVertically) {
        EntryIcon(row)
        Spacer(Modifier.width(TetherSpace.space3))
        Text(
            text = entry.name,
            style = TetherTheme.monoLabel,
            color = if (entry.ignored) TetherTheme.colors.text3 else TetherTheme.colors.text,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )
        if (sizeText != null) {
            Text(
                text = sizeText,
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        if (entry.ignored) {
            Spacer(Modifier.width(TetherSpace.space2))
            StatusBadge(label = stringResource(R.string.files_ignored), tone = BadgeTone.Neutral)
        }
    }
}

/** The row's touch target and indent. A folder is announced as expanded or collapsed. */
@Composable
private fun Modifier.entryRowModifier(
    row: TreeRow.Entry,
    onToggle: (String) -> Unit,
    onOpenFile: (String) -> Unit,
): Modifier {
    val entry = row.entry
    val expandedLabel = stringResource(R.string.files_expanded)
    val collapsedLabel = stringResource(R.string.files_collapsed)
    val stateLabel = if (row.expanded) expandedLabel else collapsedLabel
    return this
        .fillMaxWidth()
        .defaultMinSize(minHeight = TetherSize.touchMin)
        .padding(start = indentFor(row.depth), end = TetherSpace.space4)
        .semantics {
            if (entry.isDir) stateDescription = stateLabel
        }.clickable(role = Role.Button) {
            if (entry.isDir) onToggle(entry.path) else onOpenFile(entry.path)
        }
}

/** The folder icon, open or closed, or the file icon for a file. */
@Composable
private fun EntryIcon(row: TreeRow.Entry) {
    val icon =
        when {
            !row.entry.isDir -> TetherIcons.FileText
            row.expanded -> TetherIcons.FolderOpen
            else -> TetherIcons.Folder
        }
    TetherIcon(
        icon = icon,
        contentDescription = null,
        tint = if (row.entry.ignored) TetherTheme.colors.text3 else TetherTheme.colors.text2,
        size = TetherSize.iconS,
    )
}

/** A thin indeterminate line that marks a load in progress. [label] is what TalkBack reads. */
@Composable
fun ThinLoadingLine(
    label: String,
    modifier: Modifier = Modifier,
) {
    LinearProgressIndicator(
        modifier = modifier.fillMaxWidth().semantics { contentDescription = label },
        color = TetherTheme.colors.accent,
        trackColor = TetherTheme.colors.surfaceSunken,
    )
}

/** The hairline between two rows of one list group. */
@Composable
fun FileDivider() {
    HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
}
