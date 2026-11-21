package app.tether.home

import app.tether.session.SessionGateway

/**
 * Release builds read the real projects and sessions. Usage stays empty until the core reports
 * real usage, so the Home cards show the honest empty state for it.
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
