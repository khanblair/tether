package app.tether.settings.github

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * The signed-in account's repositories: a client-side search, a count, the rows, and Load more.
 * [selectedUrl] marks the row that the URL field holds. Picking a row fills the URL.
 */
@Composable
fun CloneRepoList(
    repos: List<GithubRepo>,
    query: String,
    selectedUrl: String,
    hasMore: Boolean,
    loading: Boolean,
    failure: GithubFailure?,
    onQueryChange: (String) -> Unit,
    onPick: (GithubRepo) -> Unit,
    onLoadMore: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val visible = filterRepos(repos, query)
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        SectionTitle(text = stringResource(R.string.github_clone_repos_title))
        TetherSearchField(
            value = query,
            onValueChange = onQueryChange,
            placeholder = stringResource(R.string.github_clone_search_placeholder),
            clearLabel = stringResource(R.string.github_clone_search_clear),
        )
        if (repos.isNotEmpty()) {
            Text(
                text = countText(repos.size, visible.size, query),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        failure?.let {
            InlineBanner(
                text = stringResource(it.messageRes()),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.github_clone_try_again),
                onAction = onLoadMore,
            )
        }
        when {
            loading && repos.isEmpty() ->
                Text(
                    text = stringResource(R.string.github_clone_loading),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text2,
                )
            repos.isEmpty() && failure == null ->
                Text(
                    text = stringResource(R.string.github_clone_repos_empty),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text2,
                )
            visible.isEmpty() && query.isNotBlank() ->
                Text(
                    text = stringResource(R.string.github_clone_no_match, query.trim()),
                    style = MaterialTheme.typography.bodyMedium,
                    color = TetherTheme.colors.text2,
                )
            else ->
                ListGroup {
                    visible.forEachIndexed { index, repo ->
                        if (index > 0) {
                            HorizontalDivider(
                                thickness = TetherBorder.hairline,
                                color = TetherTheme.colors.line,
                            )
                        }
                        RepoRow(
                            repo = repo,
                            selected = repo.cloneUrl == selectedUrl,
                            onClick = { onPick(repo) },
                        )
                    }
                }
        }
        if (hasMore && !loading) {
            TetherTextButton(
                label = stringResource(R.string.github_clone_load_more),
                onClick = onLoadMore,
            )
        }
    }
}

/** One repository: its slug, privacy, and size, with a check when the URL field holds it. */
@Composable
private fun RepoRow(
    repo: GithubRepo,
    selected: Boolean,
    onClick: () -> Unit,
) {
    val privacy =
        stringResource(
            if (repo.isPrivate) R.string.github_repo_private else R.string.github_repo_public,
        )
    ListRow(
        title = repoSlug(repo.cloneUrl) ?: repo.cloneUrl,
        subtitle =
            stringResource(
                R.string.github_repo_summary,
                privacy,
                megabytesFromKb(repo.sizeKb).toInt(),
            ),
        onClick = onClick,
        trailing =
            if (selected) {
                {
                    TetherIcon(
                        icon = TetherIcons.CircleCheck,
                        contentDescription = stringResource(R.string.github_repo_selected),
                        tint = TetherTheme.colors.accentInk,
                    )
                }
            } else {
                null
            },
    )
}

/** The count line: all repositories, or how many match the search. */
@Composable
private fun countText(
    total: Int,
    matches: Int,
    query: String,
): String =
    if (query.isBlank()) {
        pluralStringResource(R.plurals.github_clone_repo_count, total, total)
    } else {
        pluralStringResource(R.plurals.github_clone_repo_count, matches, matches)
    }

/** The primary Clone button, shown with the exact URL it will clone. */
@Composable
internal fun CloneTarget(
    url: String,
    enabled: Boolean,
    onClone: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        if (url.isNotBlank()) {
            Text(
                text = stringResource(R.string.github_clone_cloning_target, url),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        PrimaryButton(
            label = stringResource(R.string.github_clone_button),
            onClick = onClone,
            enabled = enabled,
        )
    }
}
