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
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
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
import app.tether.design.ThemeMode
import app.tether.design.components.DrawerItem
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.SessionRow
import app.tether.design.components.SessionState
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSearchField
import app.tether.design.components.ThemeToggleButton
import app.tether.design.next
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace
import app.tether.nav.Route
import app.tether.session.sessionTitleText

/** A top-level destination in the drawer. */
private data class Destination(
    val route: Route,
    @StringRes val label: Int,
    val icon: ImageVector,
)

private val DESTINATIONS =
    listOf(
        Destination(Route.Home, R.string.drawer_home, TetherIcons.House),
        Destination(Route.Projects, R.string.drawer_projects, TetherIcons.Folder),
        Destination(Route.Attention, R.string.drawer_attention, TetherIcons.Bell),
        Destination(Route.Cost, R.string.drawer_usage, TetherIcons.ChartColumn),
        Destination(Route.Runs, R.string.drawer_runs, TetherIcons.Activity),
    )

/**
 * The navigation drawer. The header holds the theme toggle. The list holds the top-level
 * destinations, then the session search and the sessions grouped by project. The actions stay
 * pinned at the bottom. [now] is the time the relative times are taken from.
 */
@Composable
fun HomeDrawer(
    sessions: List<SessionSummary>,
    query: String,
    onQueryChange: (String) -> Unit,
    currentRoute: String?,
    attentionCount: Int,
    themeMode: ThemeMode,
    onToggleTheme: () -> Unit,
    onDestination: (Route) -> Unit,
    onOpenSession: (String) -> Unit,
    onRenameSession: (SessionSummary) -> Unit,
    onNewSession: () -> Unit,
    onOpenSettings: () -> Unit,
    onOpenSkills: () -> Unit,
    now: Long,
    onSearchMessages: ((String) -> Unit)? = null,
) {
    ModalDrawerSheet(
        modifier = Modifier.width(TetherSize.paneList),
        drawerContainerColor = TetherTheme.colors.surface,
        drawerContentColor = TetherTheme.colors.text,
    ) {
        Column(modifier = Modifier.fillMaxSize()) {
            DrawerHeader(themeMode = themeMode, onToggleTheme = onToggleTheme)
            DrawerList(
                sessions = sessions,
                query = query,
                onQueryChange = onQueryChange,
                currentRoute = currentRoute,
                attentionCount = attentionCount,
                onDestination = onDestination,
                onOpenSession = onOpenSession,
                onRenameSession = onRenameSession,
                onSearchMessages = onSearchMessages,
                now = now,
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
private fun DrawerHeader(
    themeMode: ThemeMode,
    onToggleTheme: () -> Unit,
) {
    val current = themeModeName(themeMode)
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
            modifier = Modifier.weight(1f),
        )
        ThemeToggleButton(
            mode = themeMode,
            contentDescription =
                stringResource(
                    R.string.drawer_theme_description,
                    current,
                    themeModeName(themeMode.next()),
                ),
            onClick = onToggleTheme,
        )
    }
}

@Composable
private fun DrawerList(
    sessions: List<SessionSummary>,
    query: String,
    onQueryChange: (String) -> Unit,
    currentRoute: String?,
    attentionCount: Int,
    onDestination: (Route) -> Unit,
    onOpenSession: (String) -> Unit,
    onRenameSession: (SessionSummary) -> Unit,
    onSearchMessages: ((String) -> Unit)?,
    now: Long,
    modifier: Modifier,
) {
    val groups = remember(sessions, query) { groupSessions(sessions, query) }
    LazyColumn(modifier = modifier.fillMaxWidth()) {
        destinationItems(currentRoute, attentionCount, onDestination)
        item {
            HorizontalDivider(
                modifier = Modifier.padding(vertical = TetherSpace.space2),
                thickness = TetherBorder.hairline,
                color = TetherTheme.colors.line,
            )
        }
        item {
            TetherSearchField(
                value = query,
                onValueChange = onQueryChange,
                placeholder = stringResource(R.string.drawer_search_placeholder),
                clearLabel = stringResource(R.string.home_search_clear),
                modifier = Modifier.padding(horizontal = TetherSpace.space4),
            )
        }
        if (onSearchMessages != null && query.isNotBlank()) {
            item {
                ListRow(
                    title = stringResource(R.string.drawer_search_all, query.trim()),
                    leading = { DrawerIcon(TetherIcons.Search) },
                    onClick = { onSearchMessages(query.trim()) },
                )
            }
        }
        if (groups.isEmpty() && query.isNotBlank()) {
            item { NoSessionMatch(query = query) }
        }
        groups.forEach { (project, rows) ->
            item(key = "group-$project") { SectionTitle(text = project) }
            items(rows, key = { it.id }) { session ->
                SessionRow(
                    title = sessionTitleText(session.title),
                    subtitle = sessionSubtitle(session, now),
                    state = session.state,
                    onClick = { onOpenSession(session.id) },
                    onLongClick = { onRenameSession(session) },
                )
            }
        }
    }
}

private fun LazyListScope.destinationItems(
    currentRoute: String?,
    attentionCount: Int,
    onDestination: (Route) -> Unit,
) {
    items(DESTINATIONS, key = { it.route.path }) { destination ->
        DrawerItem(
            label = stringResource(destination.label),
            icon = destination.icon,
            selected = currentRoute == destination.route.path,
            onClick = { onDestination(destination.route) },
            badge = destination.badgeFor(attentionCount),
        )
    }
}

/** The count shown beside Needs attention, when anything needs the user. */
private fun Destination.badgeFor(attentionCount: Int): String? =
    if (route == Route.Attention && attentionCount > 0) attentionCount.toString() else null

@Composable
private fun themeModeName(mode: ThemeMode): String =
    stringResource(
        when (mode) {
            ThemeMode.System -> R.string.settings_theme_system
            ThemeMode.Light -> R.string.settings_theme_light
            ThemeMode.Dark -> R.string.settings_theme_dark
        },
    )

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
