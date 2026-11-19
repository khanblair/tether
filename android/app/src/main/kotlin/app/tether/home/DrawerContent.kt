package app.tether.home

import androidx.annotation.StringRes
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.SessionRow
import app.tether.design.components.SessionState
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSearchField
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The navigation drawer: sessions grouped by project, with the search, All projects, and the
 * actions pinned at the bottom. Every item is a plain row or button, so the drawer never needs a
 * bottom bar. [now] is the time the relative times are taken from.
 */
@Composable
fun HomeDrawer(
    sessions: List<SessionSummary>,
    query: String,
    onQueryChange: (String) -> Unit,
    onOpenSession: (String) -> Unit,
    onAllProjects: () -> Unit,
    onNewSession: () -> Unit,
    onOpenSettings: () -> Unit,
    onOpenSkills: () -> Unit,
    now: Long,
) {
    ModalDrawerSheet(
        modifier = Modifier.width(TetherSize.paneList),
        drawerContainerColor = TetherTheme.colors.surface,
        drawerContentColor = TetherTheme.colors.text,
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            DrawerHeader()
            TetherSearchField(
                value = query,
                onValueChange = onQueryChange,
                placeholder = stringResource(R.string.drawer_search_placeholder),
                clearLabel = stringResource(R.string.home_search_clear),
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
            SessionList(
                sessions = sessions,
                query = query,
                now = now,
                onOpenSession = onOpenSession,
                onAllProjects = onAllProjects,
                modifier = Modifier.weight(1f),
            )
            DrawerFooter(
                onNewSession = onNewSession,
                onOpenSettings = onOpenSettings,
                onOpenSkills = onOpenSkills,
            )
        }
    }
}

@Composable
private fun DrawerHeader() {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.statusBars)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Image(
            painter = painterResource(R.drawable.ic_launcher_monochrome),
            contentDescription = null,
            modifier = Modifier.width(TetherSize.icon),
            colorFilter = ColorFilter.tint(TetherTheme.colors.text),
        )
        Spacer(Modifier.width(TetherSpace.space2))
        Text(
            text = stringResource(R.string.about_app_name),
            style = MaterialTheme.typography.titleMedium,
            color = TetherTheme.colors.text,
        )
    }
}

@Composable
private fun SessionList(
    sessions: List<SessionSummary>,
    query: String,
    now: Long,
    onOpenSession: (String) -> Unit,
    onAllProjects: () -> Unit,
    modifier: Modifier,
) {
    val groups = remember(sessions, query) { groupSessions(sessions, query) }
    LazyColumn(modifier = modifier.fillMaxWidth()) {
        if (groups.isEmpty() && query.isNotBlank()) {
            item { NoSessionMatch(query = query) }
        }
        groups.forEach { (project, rows) ->
            item(key = "group-$project") { SectionTitle(text = project) }
            items(rows, key = { it.id }) { session ->
                SessionRow(
                    title = session.title,
                    subtitle = sessionSubtitle(session, now),
                    state = session.state,
                    onClick = { onOpenSession(session.id) },
                )
            }
        }
        item {
            ListRow(
                title = stringResource(R.string.drawer_all_projects),
                leading = { DrawerIcon(TetherIcons.Folder) },
                onClick = onAllProjects,
            )
        }
    }
}

@Composable
private fun NoSessionMatch(query: String) {
    Text(
        text = stringResource(R.string.drawer_search_empty, query.trim()),
        style = MaterialTheme.typography.bodyLarge,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
    )
}

@Composable
private fun sessionSubtitle(
    session: SessionSummary,
    now: Long,
): String {
    val ago = relativeTime(now, session.updatedAt).text()
    val pullRequest = session.pullRequest
    return when (session.state) {
        SessionState.Running -> stringResource(R.string.drawer_state_running)
        SessionState.Waiting -> stringResource(R.string.drawer_state_waiting)
        SessionState.Failed -> stringResource(R.string.drawer_state_stopped, ago)
        SessionState.Done ->
            if (pullRequest != null) {
                stringResource(R.string.drawer_state_pr, pullRequest)
            } else {
                stringResource(R.string.drawer_state_done, ago)
            }
        SessionState.Idle -> ago
    }
}

@Composable
private fun DrawerFooter(
    onNewSession: () -> Unit,
    onOpenSettings: () -> Unit,
    onOpenSkills: () -> Unit,
) {
    Column(
        modifier =
            Modifier
                .fillMaxWidth()
                .windowInsetsPadding(WindowInsets.navigationBars)
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        PrimaryButton(
            label = stringResource(R.string.drawer_new_session),
            onClick = onNewSession,
            modifier = Modifier.fillMaxWidth(),
        )
        DrawerRow(
            title = R.string.drawer_settings,
            icon = TetherIcons.Settings,
            onClick = onOpenSettings,
        )
        DrawerRow(title = R.string.drawer_skills, icon = TetherIcons.Brain, onClick = onOpenSkills)
    }
}

@Composable
private fun DrawerRow(
    @StringRes title: Int,
    icon: ImageVector,
    onClick: () -> Unit,
) {
    ListRow(
        title = stringResource(title),
        leading = { DrawerIcon(icon) },
        onClick = onClick,
    )
}

@Composable
private fun DrawerIcon(icon: ImageVector) {
    TetherIcon(icon = icon, contentDescription = null, tint = TetherTheme.colors.text2)
}
