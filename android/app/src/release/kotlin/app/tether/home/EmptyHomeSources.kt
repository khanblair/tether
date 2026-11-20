package app.tether.home

/** Release builds show the honest empty state until the core reports real projects. */
fun createHomeSources(): HomeSources =
    HomeSources(
        projects =
            object : ProjectsSource {
                override fun projects(): List<ProjectSummary> = emptyList()
            },
        sessions =
            object : SessionsSource {
                override fun sessions(): List<SessionSummary> = emptyList()
            },
        usage =
            object : UsageSource {
                override fun usage(): UsageSnapshot = UsageSnapshot.Empty
            },
    )
