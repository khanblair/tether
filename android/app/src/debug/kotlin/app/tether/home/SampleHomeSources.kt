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
            ProjectSummary("acme-api", "acme-api", "main", 3, ago(12.minutes), running = true),
            ProjectSummary("tether", "tether", "main", 0, ago(2.hours), running = false),
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
            ),
            SessionSummary(
                "s2",
                "Migrate to Pydantic v2",
                "acme-api",
                SessionState.Waiting,
                ago(5.minutes),
            ),
            SessionSummary(
                "s3",
                "Add rate limit to /v1/search",
                "acme-api",
                SessionState.Done,
                ago(8.minutes),
                pullRequest = 42,
            ),
            SessionSummary(
                "s4",
                "Add UniFFI event path",
                "tether",
                SessionState.Done,
                ago(2.hours),
            ),
            SessionSummary(
                "s5",
                "Run the test suite",
                "blog",
                SessionState.Failed,
                ago(12.minutes),
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
    )
}
