package app.tether.home

import app.tether.core.generated.SessionStatusView
import app.tether.design.components.SessionState
import app.tether.session.FakeSessionGateway
import app.tether.session.sessionInfo
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CoreHomeSourcesTest {
    private val projects =
        FakeProjectGateway(
            listOf(
                ProjectInfo("api", "acme-api", lastActivityMs = 5_000L),
                ProjectInfo("blog", "blog", lastActivityMs = 3_000L),
            ),
        )
    private val sessions = FakeSessionGateway()

    @Test
    fun every_core_status_maps_to_one_list_state() {
        val expected =
            mapOf(
                SessionStatusView.IDLE to SessionState.Idle,
                SessionStatusView.REQUESTING to SessionState.Running,
                SessionStatusView.STREAMING to SessionState.Running,
                SessionStatusView.CHECKING_TOOL to SessionState.Running,
                SessionStatusView.RUNNING_TOOL to SessionState.Running,
                SessionStatusView.AWAITING_APPROVAL to SessionState.Waiting,
                SessionStatusView.PAUSED to SessionState.Waiting,
                SessionStatusView.INTERRUPTED to SessionState.Failed,
                SessionStatusView.FAILED to SessionState.Failed,
            )
        assertEquals(SessionStatusView.entries.size, expected.size)
        expected.forEach { (status, state) -> assertEquals(state, status.toSessionState()) }
    }

    @Test
    fun a_project_is_running_when_one_of_its_sessions_is_running() {
        sessions.sessionsByProject["api"] =
            listOf(
                sessionInfo("a", SessionStatusView.IDLE),
                sessionInfo("b", SessionStatusView.STREAMING),
            )
        sessions.sessionsByProject["blog"] = listOf(sessionInfo("c", SessionStatusView.FAILED))

        val summaries = CoreProjectsSource(projects, sessions).projects()

        assertEquals(listOf("acme-api", "blog"), summaries.map { it.name })
        assertTrue(summaries[0].running)
        assertFalse(summaries[1].running)
        assertEquals("", summaries[0].branch)
        assertEquals(0, summaries[0].uncommitted)
        assertEquals(5_000L, summaries[0].lastActivityAt)
    }

    @Test
    fun sessions_carry_their_project_name_id_and_state() {
        sessions.sessionsByProject["blog"] =
            listOf(
                sessionInfo(
                    "c",
                    SessionStatusView.AWAITING_APPROVAL,
                    title = "Fix tests",
                    updatedAtMs = 9_000L,
                ),
            )

        val summaries = CoreSessionsSource(projects, sessions).sessions()

        assertEquals(1, summaries.size)
        val session = summaries.single()
        assertEquals("c", session.id)
        assertEquals("Fix tests", session.title)
        assertEquals("blog", session.projectName)
        assertEquals("blog", session.projectId)
        assertEquals(SessionState.Waiting, session.state)
        assertEquals(9_000L, session.updatedAt)
    }

    @Test
    fun archived_sessions_map_their_fields_and_project_per_project() {
        sessions.archivedByProject["api"] =
            listOf(
                sessionInfo("a", SessionStatusView.FAILED, title = "Old fix", updatedAtMs = 4_000L),
            )
        sessions.archivedByProject["blog"] =
            listOf(
                sessionInfo("c", SessionStatusView.IDLE, title = "Old post", updatedAtMs = 2_000L),
            )

        val archived = CoreSessionsSource(projects, sessions).archivedSessions()

        assertEquals(listOf("a", "c"), archived.map { it.id })
        val first = archived[0]
        assertEquals("Old fix", first.title)
        assertEquals("acme-api", first.projectName)
        assertEquals("api", first.projectId)
        assertEquals(SessionState.Failed, first.state)
        assertEquals(4_000L, first.updatedAt)
        assertEquals("blog", archived[1].projectId)
    }

    @Test
    fun archived_sessions_are_not_in_the_active_list() {
        sessions.sessionsByProject["blog"] = listOf(sessionInfo("live", SessionStatusView.IDLE))
        sessions.archivedByProject["blog"] = listOf(sessionInfo("old", SessionStatusView.IDLE))

        val active = CoreSessionsSource(projects, sessions).sessions()

        assertEquals(listOf("live"), active.map { it.id })
    }

    @Test
    fun no_archived_sessions_gives_an_empty_list() {
        assertTrue(CoreSessionsSource(projects, sessions).archivedSessions().isEmpty())
    }

    @Test
    fun a_fresh_install_has_no_projects_and_no_sessions() {
        val empty = FakeProjectGateway()
        assertTrue(CoreProjectsSource(empty, sessions).projects().isEmpty())
        assertTrue(CoreSessionsSource(empty, sessions).sessions().isEmpty())
        assertTrue(CoreSessionsSource(empty, sessions).archivedSessions().isEmpty())
    }
}
