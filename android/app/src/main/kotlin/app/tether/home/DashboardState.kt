package app.tether.home

import androidx.compose.runtime.Immutable
import app.tether.core.generated.ProviderView
import app.tether.design.components.SessionState
import app.tether.updates.UpdatesUiState
import java.time.Instant
import java.time.ZoneId
import java.time.format.TextStyle
import java.util.Locale

private const val RECENT_PROJECTS = 3

/** The one sentence at the top of Home. */
sealed interface SummaryLine {
    /** Items need the user. [running] counts the sessions running now. */
    data class NeedsYou(
        val needs: Int,
        val running: Int,
    ) : SummaryLine

    /** Nothing needs the user. [running] counts the sessions running now. */
    data class Calm(
        val running: Int,
    ) : SummaryLine

    /** A new install: no key, no project, and no usage yet. */
    data object Welcome : SummaryLine
}

/** One provider and whether a key is saved for it. */
data class ProviderReady(
    val provider: ProviderView,
    val hasKey: Boolean,
)

/**
 * Everything the Home cards show. [providers] is null until the saved keys are read. [now] is the
 * time the relative times are taken from.
 */
@Immutable
data class DashboardUiState(
    val summary: SummaryLine,
    val attention: List<AttentionItem>,
    val running: List<SessionSummary>,
    val usage: UsageSnapshot,
    val sessionsToday: Change,
    val tokensToday: Change,
    val filesToday: Change,
    val weekChange: Change,
    val dayInitials: List<String>,
    val dayNames: List<String>,
    val shares: List<ModelShare>,
    val recentProjects: List<ProjectSummary>,
    val projectCount: Int,
    val providers: List<ProviderReady>?,
    val now: Long,
)

/** Builds the Home state from the loaded data. Pure, so the tests need no Android. */
@Suppress("LongParameterList")
fun dashboardState(
    projects: List<ProjectSummary>,
    sessions: List<SessionSummary>,
    usage: UsageSnapshot,
    updates: UpdatesUiState,
    providers: List<ProviderReady>?,
    now: Long,
    zone: ZoneId,
    locale: Locale,
): DashboardUiState {
    val attention = attentionItems(sessions, updates)
    val running =
        sessions
            .filter { it.state == SessionState.Running }
            .sortedByDescending { it.updatedAt }
    val today = Instant.ofEpochMilli(now).atZone(zone).toLocalDate()
    return DashboardUiState(
        summary = summaryLine(projects, usage, providers, attention, running),
        attention = attention,
        running = running,
        usage = usage,
        sessionsToday = countChange(usage.today.sessions, usage.yesterday.sessions),
        tokensToday = percentChange(usage.today.tokens, usage.yesterday.tokens),
        filesToday = countChange(usage.today.filesChanged, usage.yesterday.filesChanged),
        weekChange = percentChange(usage.weekTokens, usage.previousWeekTokens),
        dayInitials = weekdayLabels(today, locale, TextStyle.NARROW),
        dayNames = weekdayLabels(today, locale, TextStyle.FULL),
        shares = modelShares(usage.byModel),
        recentProjects = projects.sortedByDescending { it.lastActivityAt }.take(RECENT_PROJECTS),
        projectCount = projects.size,
        providers = providers,
        now = now,
    )
}

private fun summaryLine(
    projects: List<ProjectSummary>,
    usage: UsageSnapshot,
    providers: List<ProviderReady>?,
    attention: List<AttentionItem>,
    running: List<SessionSummary>,
): SummaryLine {
    val noKeys = providers != null && providers.none { it.hasKey }
    return when {
        noKeys && projects.isEmpty() && !usage.hasUsage -> SummaryLine.Welcome
        attention.isNotEmpty() -> SummaryLine.NeedsYou(attention.size, running.size)
        else -> SummaryLine.Calm(running.size)
    }
}
