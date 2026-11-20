package app.tether.home

import app.tether.core.generated.ProviderView
import app.tether.design.components.SessionState
import app.tether.updates.UpdatesUiState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.time.Instant
import java.time.ZoneId
import java.util.Locale

class DashboardStateTest {
    private val now = Instant.parse("2026-11-19T12:00:00Z").toEpochMilli()
    private val noUpdate = UpdatesUiState(currentVersion = "0.0.3", installsInApp = true)
    private val noKeys = ProviderView.entries.map { ProviderReady(it, hasKey = false) }
    private val oneKey = noKeys.mapIndexed { index, ready -> ready.copy(hasKey = index == 0) }

    private fun session(
        id: String,
        state: SessionState,
        updatedAt: Long = now,
    ) = SessionSummary(id, "title $id", "acme", state, updatedAt)

    private fun project(
        id: String,
        lastActivityAt: Long,
    ) = ProjectSummary(id, id, "main", 0, lastActivityAt, running = false)

    private fun build(
        projects: List<ProjectSummary> = emptyList(),
        sessions: List<SessionSummary> = emptyList(),
        usage: UsageSnapshot = UsageSnapshot.Empty,
        updates: UpdatesUiState = noUpdate,
        providers: List<ProviderReady>? = oneKey,
    ) = dashboardState(
        projects = projects,
        sessions = sessions,
        usage = usage,
        updates = updates,
        providers = providers,
        now = now,
        zone = ZoneId.of("UTC"),
        locale = Locale.US,
    )

    private val sampleUsage =
        UsageSnapshot(
            today = DayTotals(sessions = 4, tokens = 212_000L, filesChanged = 17),
            yesterday = DayTotals(sessions = 2, tokens = 180_000L, filesChanged = 17),
            lastSevenDays =
                listOf(
                    120_000L,
                    150_000L,
                    95_000L,
                    240_000L,
                    160_000L,
                    180_000L,
                    212_000L,
                ),
            previousWeekTokens = 1_000_000L,
            byModel = listOf(ModelUsage("Sonnet 5.5", 717_000L)),
        )

    @Test
    fun a_project_ends_the_welcome_even_with_no_keys_and_no_usage() {
        val state = build(projects = listOf(project("p", now)), providers = noKeys)
        assertEquals(SummaryLine.Calm(running = 0), state.summary)
    }

    @Test
    fun a_new_install_with_no_keys_projects_or_usage_is_welcome() {
        assertEquals(SummaryLine.Welcome, build(providers = noKeys).summary)
    }

    @Test
    fun welcome_needs_the_keys_to_be_read_and_none_saved() {
        assertEquals(SummaryLine.Calm(0), build(providers = null).summary)
        assertEquals(SummaryLine.Calm(0), build(providers = oneKey).summary)
    }

    @Test
    fun an_attention_item_gives_needs_you_with_the_running_count() {
        val sessions =
            listOf(
                session("a", SessionState.Waiting),
                session("b", SessionState.Running),
                session("c", SessionState.Running),
            )
        assertEquals(
            SummaryLine.NeedsYou(needs = 1, running = 2),
            build(sessions = sessions).summary,
        )
    }

    @Test
    fun with_nothing_needing_the_user_the_summary_is_calm() {
        val sessions = listOf(session("a", SessionState.Running))
        assertEquals(SummaryLine.Calm(running = 1), build(sessions = sessions).summary)
    }

    @Test
    fun the_recent_projects_are_the_three_newest() {
        val projects =
            listOf(
                project("old", 1_000L),
                project("newest", 9_000L),
                project("second", 8_000L),
                project("third", 7_000L),
                project("fourth", 2_000L),
            )
        val state = build(projects = projects)
        assertEquals(listOf("newest", "second", "third"), state.recentProjects.map { it.id })
        assertEquals(5, state.projectCount)
    }

    @Test
    fun the_running_list_holds_running_sessions_newest_first() {
        val sessions =
            listOf(
                session("older", SessionState.Running, updatedAt = now - 5_000L),
                session("done", SessionState.Done),
                session("newer", SessionState.Running, updatedAt = now - 1_000L),
            )
        assertEquals(listOf("newer", "older"), build(sessions = sessions).running.map { it.id })
    }

    @Test
    fun an_update_appears_in_attention() {
        val update =
            UpdatesUiState(
                currentVersion = "0.0.3",
                installsInApp = true,
                knownNewerVersion = "0.0.4",
            )
        val state = build(updates = update)
        assertEquals(listOf(AttentionItem.Update("0.0.4", "0.0.3")), state.attention)
        assertEquals(SummaryLine.NeedsYou(needs = 1, running = 0), state.summary)
    }

    @Test
    fun the_today_changes_compare_with_yesterday() {
        val state = build(usage = sampleUsage)
        assertEquals(Change.MoreCount(2), state.sessionsToday)
        assertEquals(Change.MorePercent(18), state.tokensToday)
        assertEquals(Change.Same, state.filesToday)
    }

    @Test
    fun the_week_change_compares_with_the_previous_week() {
        val state = build(usage = sampleUsage)
        assertEquals(1_157_000L, state.usage.weekTokens)
        assertEquals(Change.MorePercent(16), state.weekChange)
    }

    @Test
    fun the_day_labels_end_with_the_day_of_now() {
        val state = build(usage = sampleUsage)
        assertEquals("Thursday", state.dayNames.last())
        assertEquals("T", state.dayInitials.last())
        assertEquals(7, state.dayNames.size)
    }

    @Test
    fun the_providers_stay_null_until_they_are_read() {
        val state = build(providers = null)
        assertTrue(state.providers == null)
    }

    @Test
    fun the_state_carries_the_now_time() {
        assertEquals(now, build().now)
    }
}
