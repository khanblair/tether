package app.tether.home

/**
 * Groups sessions by project for the drawer. Sessions whose title or project name contains
 * [query] (ignoring case) are kept. Groups come in the order of their most recent session, and
 * sessions inside a group are newest first.
 */
fun groupSessions(
    sessions: List<SessionSummary>,
    query: String,
): List<Pair<String, List<SessionSummary>>> {
    val needle = query.trim()
    return sessions
        .filter { session ->
            needle.isEmpty() ||
                session.title.contains(needle, ignoreCase = true) ||
                session.projectName.contains(needle, ignoreCase = true)
        }.sortedByDescending { it.updatedAt }
        .groupBy { it.projectName }
        .toList()
}
