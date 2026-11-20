package app.tether.home

import app.tether.design.components.SessionState
import kotlin.time.Duration.Companion.days
import kotlin.time.Duration.Companion.hours
import kotlin.time.Duration.Companion.minutes

/** Debug builds show sample projects and sessions, so the screens have something to show. */
fun createHomeSources(): HomeSources {
    val now = System.currentTimeMillis()

    fun ago(duration: kotlin.time.Duration): Long = now - duration.inWholeMilliseconds
    val projects =
        listOf(
            ProjectSummary(
                "acme-api", "acme-api", "main", 3, ago(12.minutes), running = true,
                executor = ProjectExecutor.Sandbox,
            ),
            ProjectSummary(
                "tether", "tether", "main", 0, ago(2.hours), running = false,
                executor = ProjectExecutor.Ci,
            ),
            ProjectSummary("dotfiles", "dotfiles", "feat/zsh", 1, ago(30.hours), running = false),
            ProjectSummary("blog", "blog", "main", 0, ago(3.days), running = false),
        )
    val sessions =
        listOf(
            SessionSummary(
                "s1",
                "Fix login timeout",
                "acme-api",
                SessionState.Running,
                ago(1.minutes),
                projectId = "acme-api",
            ),
            SessionSummary(
                "s2",
                "Migrate to Pydantic v2",
                "acme-api",
                SessionState.Waiting,
                ago(5.minutes),
                projectId = "acme-api",
            ),
            SessionSummary(
                "s3",
                "Add rate limit to /v1/search",
                "acme-api",
                SessionState.Done,
                ago(8.minutes),
                pullRequest = 42,
                projectId = "acme-api",
            ),
            SessionSummary(
                "s4",
                "Add UniFFI event path",
                "tether",
                SessionState.Done,
                ago(2.hours),
                projectId = "tether",
            ),
            SessionSummary(
                "s5",
                "Run the test suite",
                "blog",
                SessionState.Failed,
                ago(12.minutes),
                projectId = "blog",
            ),
        )
    return HomeSources(
        projects =
            object : ProjectsSource {
                override fun projects(): List<ProjectSummary> = projects
            },
        sessions =
            object : SessionsSource {
                override fun sessions(): List<SessionSummary> = sessions
            },
        usage = SampleUsage,
    )
}

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
