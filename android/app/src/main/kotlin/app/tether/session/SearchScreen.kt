package app.tether.session

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.TetherSearchField
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.shell.BackButton

/**
 * Searches messages across sessions. [hasProject] shows the "This project" scope; without a
 * project in view, only "All projects" is offered. A tap on a hit opens its session.
 */
@Composable
fun SearchScreen(
    viewModel: SearchViewModel,
    hasProject: Boolean,
    onBack: () -> Unit,
    onOpenSession: (String) -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(R.string.session_search_title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            TetherSearchField(
                value = state.query,
                onValueChange = viewModel::onQueryChange,
                placeholder = stringResource(R.string.session_search_placeholder),
                clearLabel = stringResource(R.string.home_search_clear),
            )
            if (hasProject) ScopeChips(state.scope, viewModel::setScope)
            SearchBody(state, onOpenSession)
        }
    }
}

@Composable
private fun ScopeChips(
    selected: SearchScope,
    onScope: (SearchScope) -> Unit,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        FilterChip(
            selected = selected == SearchScope.THIS_PROJECT,
            onClick = { onScope(SearchScope.THIS_PROJECT) },
            label = { Text(stringResource(R.string.session_search_scope_project)) },
        )
        FilterChip(
            selected = selected == SearchScope.ALL_PROJECTS,
            onClick = { onScope(SearchScope.ALL_PROJECTS) },
            label = { Text(stringResource(R.string.session_search_scope_all)) },
        )
    }
}

/** The result area: a hint before any query, the hits, or one line when nothing matched. */
@Composable
private fun ColumnScope.SearchBody(
    state: SearchState,
    onOpenSession: (String) -> Unit,
) {
    when {
        state.failure != null ->
            InlineBanner(
                text = stringResource(R.string.session_error_search),
                tone = BannerTone.Danger,
            )
        state.query.isBlank() -> BodyLine(stringResource(R.string.session_search_hint))
        state.searched && state.hits.isEmpty() && !state.loading ->
            BodyLine(stringResource(R.string.session_search_none))
        else ->
            LazyColumn(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
            ) {
                itemsIndexed(
                    state.hits,
                    key = { index, hit -> "$index-${hit.sessionId}" },
                ) { _, hit ->
                    HitRow(hit, state.query, onClick = { onOpenSession(hit.sessionId) })
                }
            }
    }
}

@Composable
private fun BodyLine(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodyMedium,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(vertical = TetherSpace.space3),
    )
}

/** One hit: the session title, then the matching text when it differs from the title. */
@Composable
private fun HitRow(
    hit: SearchHit,
    query: String,
    onClick: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .clickable(role = Role.Button, onClick = onClick)
                .padding(vertical = TetherSpace.space2),
    ) {
        Text(
            text = emphasize(hit.sessionTitle, query),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
        if (hit.matchText != hit.sessionTitle) {
            Text(
                text = emphasize(hit.matchText, query),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

/** The text with each match in bold. Bold, not color, marks a match for every reader. */
private fun emphasize(
    text: String,
    query: String,
): AnnotatedString =
    buildAnnotatedString {
        splitMatches(text, query).forEach { run ->
            if (run.matched) {
                withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append(run.text) }
            } else {
                append(run.text)
            }
        }
    }
