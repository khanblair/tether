package app.tether.home

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.DashboardCard
import app.tether.design.components.DataTable
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.ProjectRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.SessionRow
import app.tether.design.components.SessionState
import app.tether.design.components.ShareRow
import app.tether.design.components.StatItem
import app.tether.design.components.StatRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTextButton
import app.tether.design.components.UsageBarChart
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSize
import app.tether.providers.providerNameRes

/** How many attention items the Needs you card lists. */
private const val NEEDS_ROWS = 3

/** Needs you now: up to three attention items, then the sessions running now. Hidden when empty. */
@Composable
internal fun NeedsCard(
    state: DashboardUiState,
    onOpenAttention: () -> Unit,
    onOpenSession: (String) -> Unit,
    onOpenAbout: () -> Unit,
) {
    DashboardCard(
        title = stringResource(R.string.dashboard_needs_title),
        action = {
            TetherTextButton(
                label = stringResource(R.string.dashboard_see_all),
                onClick = onOpenAttention,
            )
        },
    ) {
        state.attention.take(NEEDS_ROWS).forEachIndexed { index, item ->
            if (index > 0) RowDivider()
            AttentionItemRow(
                item = item,
                now = state.now,
                onOpenSession = onOpenSession,
                onOpenAbout = onOpenAbout,
            )
        }
        if (state.running.isNotEmpty()) {
            SectionTitle(text = stringResource(R.string.dashboard_running_title))
            state.running.forEachIndexed { index, session ->
                if (index > 0) RowDivider()
                SessionRow(
                    title = session.title,
                    subtitle = stringResource(R.string.drawer_state_running),
                    state = SessionState.Running,
                    onClick = { onOpenSession(session.id) },
                )
            }
        }
    }
}

@Composable
private fun RowDivider() {
    HorizontalDivider(thickness = TetherBorder.hairline, color = TetherTheme.colors.line)
}

/** Today: sessions, tokens, and files changed, each with its comparison to yesterday. */
@Composable
internal fun TodayCard(state: DashboardUiState) {
    val yesterday = stringResource(R.string.dashboard_base_yesterday)
    val usage = state.usage.today
    DashboardCard(title = stringResource(R.string.dashboard_today_title)) {
        StatRow(
            items =
                listOf(
                    StatItem(
                        value = usage.sessions.toString(),
                        label = stringResource(R.string.dashboard_stat_sessions),
                        comparison = changeText(state.sessionsToday, yesterday),
                    ),
                    StatItem(
                        value = formatTokens(usage.tokens),
                        label = stringResource(R.string.dashboard_stat_tokens),
                        comparison = changeText(state.tokensToday, yesterday),
                    ),
                    StatItem(
                        value = usage.filesChanged.toString(),
                        label = stringResource(R.string.dashboard_stat_files),
                        comparison = changeText(state.filesToday, yesterday),
                    ),
                ),
        )
    }
}

/** Usage for the last seven days: the sentence, the bar chart, the models, and the data table. */
@Composable
internal fun UsageCard(state: DashboardUiState) {
    DashboardCard(title = stringResource(R.string.dashboard_usage_title)) {
        if (!state.usage.hasUsage) {
            Text(
                text = stringResource(R.string.dashboard_usage_empty),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            return@DashboardCard
        }
        val sentence = weekSentence(state.usage, state.weekChange)
        Text(
            text = sentence,
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        UsageBarChart(
            values = state.usage.lastSevenDays,
            labels = state.dayInitials,
            contentDescription = sentence,
        )
        ModelShares(state = state)
        ShowDataButton(state = state)
    }
}

@Composable
private fun ModelShares(state: DashboardUiState) {
    if (state.shares.isEmpty()) return
    val chart = TetherTheme.chart
    val colors = listOf(chart.series1, chart.series2, chart.series3, chart.series4)
    Text(
        text = stringResource(R.string.dashboard_by_model),
        style = MaterialTheme.typography.labelMedium,
        color = TetherTheme.colors.text3,
    )
    state.shares.forEachIndexed { index, share ->
        ShareRow(
            label = share.model,
            share = stringResource(R.string.dashboard_share_percent, share.percent),
            fraction = share.fraction,
            color = colors[minOf(index, colors.lastIndex)],
        )
    }
}

@Composable
private fun ShowDataButton(state: DashboardUiState) {
    var showData by rememberSaveable { mutableStateOf(false) }
    TetherTextButton(
        label =
            stringResource(
                if (showData) R.string.dashboard_hide_data else R.string.dashboard_show_data,
            ),
        onClick = { showData = !showData },
    )
    if (showData) {
        DataTable(
            headers =
                Pair(
                    stringResource(R.string.dashboard_table_day),
                    stringResource(R.string.dashboard_table_tokens),
                ),
            rows =
                state.dayNames.zip(state.usage.lastSevenDays) { day, tokens ->
                    day to formatTokens(tokens)
                },
        )
    }
}

/** Recent projects: the three most recently active, with the New project action when empty. */
@Composable
internal fun RecentProjectsCard(
    state: DashboardUiState,
    onOpenProject: (String) -> Unit,
    onOpenProjects: () -> Unit,
    onNewProject: () -> Unit,
) {
    DashboardCard(
        title = stringResource(R.string.dashboard_recent_title),
        action = {
            TetherTextButton(
                label = stringResource(R.string.dashboard_all_projects),
                onClick = onOpenProjects,
            )
        },
    ) {
        if (state.recentProjects.isEmpty()) {
            Text(
                text = stringResource(R.string.dashboard_no_projects),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            TetherTextButton(
                label = stringResource(R.string.home_new_project),
                onClick = onNewProject,
            )
            return@DashboardCard
        }
        state.recentProjects.forEachIndexed { index, project ->
            if (index > 0) RowDivider()
            val runningBadge: @Composable () -> Unit = { RunningProjectBadge() }
            ProjectRow(
                name = project.name,
                branch = project.branch,
                summary = projectSummary(project, state.now),
                onClick = { onOpenProject(project.id) },
                badge = if (project.running) runningBadge else null,
            )
        }
    }
}

/**
 * Providers: how many keys are saved, one row per provider, and Manage. With no key at all, the
 * card says sessions need one and offers Add a key. [providers] is known here, so it is not null.
 */
@Composable
internal fun ProvidersCard(
    providers: List<ProviderReady>,
    onOpenProviders: () -> Unit,
) {
    val saved = providers.count { it.hasKey }
    if (saved == 0) {
        DashboardCard(title = stringResource(R.string.dashboard_providers_title)) {
            Text(
                text = stringResource(R.string.dashboard_providers_none),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
            PrimaryButton(
                label = stringResource(R.string.dashboard_add_key),
                onClick = onOpenProviders,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        return
    }
    DashboardCard(
        title = stringResource(R.string.dashboard_providers_title),
        action = {
            TetherTextButton(
                label = stringResource(R.string.dashboard_providers_manage),
                onClick = onOpenProviders,
            )
        },
    ) {
        Text(
            text =
                pluralStringResource(
                    R.plurals.dashboard_providers_count,
                    providers.size,
                    saved,
                    providers.size,
                ),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        providers.forEachIndexed { index, ready ->
            if (index > 0) RowDivider()
            ProviderLine(ready = ready)
        }
    }
}

@Composable
private fun ProviderLine(ready: ProviderReady) {
    val colors = TetherTheme.colors
    val icon = if (ready.hasKey) TetherIcons.CircleCheck else TetherIcons.CircleDot
    val tint = if (ready.hasKey) colors.success else colors.text3
    val status =
        stringResource(
            if (ready.hasKey) {
                R.string.dashboard_provider_key_saved
            } else {
                R.string.dashboard_provider_no_key
            },
        )
    ListRow(
        title = stringResource(providerNameRes(ready.provider)),
        subtitle = status,
        leading = {
            TetherIcon(icon = icon, contentDescription = null, tint = tint, size = TetherSize.iconS)
        },
    )
}
