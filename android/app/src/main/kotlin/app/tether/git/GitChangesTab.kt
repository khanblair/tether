package app.tether.git

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherRadius
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The Changes tab: the checked-out branch, then the Staged and Not staged groups. Tapping a path
 * calls [onOpenDiff] with that path and the side it was tapped on.
 */
@Composable
fun ChangesList(
    content: ChangesContent,
    onOpenDiff: (path: String, staged: Boolean) -> Unit,
) {
    val stagedTitle = stringResource(R.string.git_section_staged)
    val unstagedTitle = stringResource(R.string.git_section_unstaged)
    val empty = content.staged.isEmpty() && content.unstaged.isEmpty()
    LazyColumn(contentPadding = PaddingValues(bottom = TetherSpace.space6)) {
        item(key = "branch") { BranchLine(content.branch) }
        if (empty) {
            item(key = "empty") { EmptyChanges() }
        }
        changeGroup(
            keyPrefix = "staged",
            title = stagedTitle,
            entries = content.staged,
            kindOf = { it.staged },
            onOpen = { entry -> onOpenDiff(entry.path, true) },
        )
        changeGroup(
            keyPrefix = "unstaged",
            title = unstagedTitle,
            entries = content.unstaged,
            kindOf = { it.unstaged },
            onOpen = { entry -> onOpenDiff(entry.path, false) },
        )
    }
}

/** One group: a section title, then one row per entry. An empty group adds nothing. */
private fun LazyListScope.changeGroup(
    keyPrefix: String,
    title: String,
    entries: List<ChangeEntry>,
    kindOf: (ChangeEntry) -> ChangeKind?,
    onOpen: (ChangeEntry) -> Unit,
) {
    if (entries.isEmpty()) return
    item(key = "$keyPrefix-title") { SectionTitle(text = title) }
    items(entries, key = { "$keyPrefix:${it.path}" }) { entry ->
        ChangeRow(
            path = entry.path,
            kind = kindOf(entry) ?: ChangeKind.Changed,
            onClick = { onOpen(entry) },
        )
    }
}

@Composable
private fun BranchLine(branch: String?) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        TetherIcon(
            icon = TetherIcons.GitBranch,
            contentDescription = null,
            tint = TetherTheme.colors.text2,
            size = TetherSize.iconS,
        )
        if (branch == null) {
            Text(
                text = stringResource(R.string.git_branch_none),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        } else {
            Text(
                text = branch,
                style = TetherTheme.mono,
                color = TetherTheme.colors.text,
                maxLines = 1,
                overflow = TextOverflow.MiddleEllipsis,
            )
        }
    }
}

@Composable
private fun EmptyChanges() {
    Text(
        text = stringResource(R.string.git_changes_empty),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space4),
    )
}

/**
 * One changed path: its state mark, then the path in mono. The directory is quiet and the file
 * name is emphasised. The whole row is one 48 dp touch target.
 */
@Composable
internal fun ChangeRow(
    path: String,
    kind: ChangeKind,
    onClick: () -> Unit,
) {
    val colors = TetherTheme.colors
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space1),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        ChangeMark(kind)
        Text(
            text = pathText(path, colors.text3, colors.text),
            style = TetherTheme.mono,
            maxLines = 1,
            overflow = TextOverflow.MiddleEllipsis,
            modifier = Modifier.weight(1f),
        )
    }
}

/** The one-letter state in a small chip. The label is read by TalkBack. */
@Composable
internal fun ChangeMark(kind: ChangeKind) {
    val colors = TetherTheme.colors
    val (foreground, background) =
        when (kind) {
            ChangeKind.Added -> colors.success to colors.successSoft
            ChangeKind.Modified -> colors.warning to colors.warningSoft
            ChangeKind.Deleted, ChangeKind.Conflicted -> colors.danger to colors.dangerSoft
            ChangeKind.Untracked -> colors.text3 to colors.surfaceSunken
            ChangeKind.Renamed, ChangeKind.Changed -> colors.text2 to colors.surfaceSunken
        }
    val label = stringResource(kind.labelRes())
    Box(
        modifier =
            Modifier
                .size(TetherSpace.space6)
                .clip(RoundedCornerShape(TetherRadius.xs))
                .background(background)
                .clearAndSetSemantics { contentDescription = label },
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = stringResource(kind.markRes()),
            style = TetherTheme.monoLabel,
            color = foreground,
        )
    }
}

/** The directory part in [dirColor] and the file name in [nameColor], both in one string. */
private fun pathText(
    path: String,
    dirColor: Color,
    nameColor: Color,
) = buildAnnotatedString {
    val slash = path.lastIndexOf('/')
    if (slash >= 0) {
        withStyle(SpanStyle(color = dirColor)) { append(path.substring(0, slash + 1)) }
    }
    withStyle(SpanStyle(color = nameColor, fontWeight = FontWeight.SemiBold)) {
        append(path.substring(slash + 1))
    }
}
