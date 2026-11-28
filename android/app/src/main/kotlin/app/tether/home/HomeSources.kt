package app.tether.home

import app.tether.session.SessionGateway

/** The projects the home screen lists. */
interface ProjectsSource {
    /** Every project, in any order. The view model sorts them. */
    fun projects(): List<ProjectSummary>
}

/** The sessions the drawer and Needs attention list. */
interface SessionsSource {
    /** Every session, in any order. The screens sort them. */
    fun sessions(): List<SessionSummary>

    /** Every archived session, in any order. They are hidden from [sessions] until restored. */
    fun archivedSessions(): List<SessionSummary>
}

/** The usage the Home dashboard charts. */
interface UsageSource {
    /** The usage for the dashboard cards. */
    fun usage(): UsageSnapshot
}

/** The data sources for the home screens, created once by the app module. */
data class HomeSources(
    val projects: ProjectsSource,
    val sessions: SessionsSource,
    val usage: UsageSource,
)

/**
 * The data sources for the home screens, the same in every build: the real projects and sessions
 * from the core. Usage stays empty until the core reports real usage, so the Home cards show the
 * honest empty state for it and never made-up numbers.
 */
fun createHomeSources(
    projects: ProjectGateway,
    sessions: SessionGateway,
): HomeSources =
    HomeSources(
        projects = CoreProjectsSource(projects, sessions),
        sessions = CoreSessionsSource(projects, sessions),
        usage =
            object : UsageSource {
                override fun usage(): UsageSnapshot = UsageSnapshot.Empty
            },
    )
