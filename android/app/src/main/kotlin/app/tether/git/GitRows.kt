package app.tether.git

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.TetherIcon
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/** The Branches tab: each local branch, the current one marked, read only. */
@Composable
fun BranchesList(content: BranchesContent) {
    if (content.branches.isEmpty()) {
        Text(
            text = stringResource(R.string.git_branches_empty),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
            modifier = Modifier.padding(vertical = TetherSpace.space4),
        )
        return
    }
    val noCommits = stringResource(R.string.git_branch_none)
    val currentLabel = stringResource(R.string.git_branch_current)
    LazyColumn {
        items(content.branches, key = { it.name }) { branch ->
            BranchRow(
                branch = branch,
                current = branch.name == content.current,
                noCommitsLabel = noCommits,
                currentLabel = currentLabel,
            )
        }
    }
}

/** One branch: name, a current marker when it is checked out, and its short commit id. */
@Composable
private fun BranchRow(
    branch: GitBranch,
    current: Boolean,
    noCommitsLabel: String,
    currentLabel: String,
) {
    val colors = TetherTheme.colors
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .padding(vertical = TetherSpace.space2),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        TetherIcon(
            icon = TetherIcons.GitBranch,
            contentDescription = null,
            tint = colors.text2,
            size = TetherSize.iconS,
        )
        Text(
            text = branch.name,
            style = TetherTheme.monoLabel,
            color = colors.text,
            maxLines = 1,
            overflow = TextOverflow.MiddleEllipsis,
            modifier = Modifier.weight(1f),
        )
        if (current) {
            Text(
                text = currentLabel,
                style = MaterialTheme.typography.bodySmall,
                color = colors.text2,
            )
            TetherIcon(
                icon = TetherIcons.CircleCheck,
                contentDescription = null,
                tint = colors.text2,
                size = TetherSize.iconS,
            )
        }
        val commit = branch.commit
        if (commit == null) {
            Text(
                text = noCommitsLabel,
                style = MaterialTheme.typography.bodySmall,
                color = colors.text3,
            )
        } else {
            Text(
                text = commit.take(GIT_SHORT_ID_LENGTH),
                style = TetherTheme.monoSmall,
                color = colors.text3,
            )
        }
    }
}

/** The History tab: the last commits, newest first. Rows are not tappable yet. */
@Composable
fun HistoryList(content: HistoryContent) {
    if (content.commits.isEmpty()) {
        Text(
            text = stringResource(R.string.git_history_empty),
            style = MaterialTheme.typography.bodyLarge,
            color = TetherTheme.colors.text2,
            modifier = Modifier.padding(vertical = TetherSpace.space4),
        )
        return
    }
    LazyColumn {
        items(content.commits, key = { it.id }) { commit ->
            CommitRow(commit = commit, nowMillis = content.nowMillis)
        }
    }
}

/** One commit: subject, then author and relative time, with the short id at the end. */
@Composable
private fun CommitRow(
    commit: GitCommit,
    nowMillis: Long,
) {
    val age = commitAge(nowMillis, commit.committedSeconds)
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .padding(vertical = TetherSpace.space2),
        verticalAlignment = Alignment.Top,
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = commit.subject,
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = stringResource(R.string.git_history_byline, commit.authorName, age.text()),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        Text(
            text = commit.shortId,
            style = TetherTheme.monoSmall,
            color = TetherTheme.colors.text3,
        )
    }
}
