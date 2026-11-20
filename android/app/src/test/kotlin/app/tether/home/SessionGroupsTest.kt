package app.tether.home

import app.tether.design.components.SessionState
import org.junit.Assert.assertEquals
import org.junit.Test

class SessionGroupsTest {
    private val sessions =
        listOf(
            session("s1", "Fix login timeout", "acme-api", updatedAt = 300L),
            session("s2", "Migrate to Pydantic v2", "acme-api", updatedAt = 100L),
            session("s3", "Add UniFFI event path", "tether", updatedAt = 200L),
            session("s4", "Run the test suite", "blog", updatedAt = 250L),
        )

    private fun session(
        id: String,
        title: String,
        project: String,
        updatedAt: Long,
    ) = SessionSummary(id, title, project, SessionState.Done, updatedAt)

    private fun ids(groups: List<Pair<String, List<SessionSummary>>>) =
        groups.map { (project, rows) -> project to rows.map { it.id } }

    @Test
    fun groups_come_in_the_order_of_their_most_recent_session() {
        val groups = groupSessions(sessions, query = "")
        assertEquals(
            listOf(
                "acme-api" to listOf("s1", "s2"),
                "blog" to listOf("s4"),
                "tether" to listOf("s3"),
            ),
            ids(groups),
        )
    }

    @Test
    fun the_query_matches_a_session_title() {
        assertEquals(listOf("acme-api" to listOf("s1")), ids(groupSessions(sessions, "login")))
    }

    @Test
    fun the_query_matches_a_session_title_ignoring_case() {
        assertEquals(listOf("acme-api" to listOf("s1")), ids(groupSessions(sessions, "LOGIN")))
    }

    @Test
    fun spaces_around_the_query_are_ignored() {
        assertEquals(listOf("acme-api" to listOf("s1")), ids(groupSessions(sessions, "  login ")))
    }

    @Test
    fun the_query_matches_a_project_name_ignoring_case() {
        assertEquals(listOf("tether" to listOf("s3")), ids(groupSessions(sessions, "TETHER")))
    }

    @Test
    fun a_query_with_no_match_gives_no_groups() {
        assertEquals(
            emptyList<Pair<String, List<SessionSummary>>>(),
            groupSessions(sessions, "xyz"),
        )
    }
}
