package app.tether.home

import app.tether.session.SessionGateway

/**
 * Debug builds read the real projects and sessions, and show sample usage numbers so the Home
 * cards have something to draw. The sample usage never appears in release builds.
 */
fun createHomeSources(
    projects: ProjectGateway,
    sessions: SessionGateway,
): HomeSources =
    HomeSources(
        projects = CoreProjectsSource(projects, sessions),
        sessions = CoreSessionsSource(projects, sessions),
        usage = SampleUsage,
    )

/** Seven days of sample tokens, with one week before them, so the Home cards have numbers. */
private object SampleUsage : UsageSource {
    private val snapshot =
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
            byModel =
                listOf(
                    ModelUsage("Sonnet 5.5", 717_000L),
                    ModelUsage("Haiku 5.5", 289_000L),
                    ModelUsage("Opus 5.5", 151_000L),
                ),
        )

    override fun usage(): UsageSnapshot = snapshot
}
