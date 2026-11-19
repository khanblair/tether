package app.tether.home

/** The projects the home screen lists. Debug builds read samples; release reads none yet. */
interface ProjectsSource {
    /** Every project, in any order. The view model sorts them. */
    fun projects(): List<ProjectSummary>
}

/** The sessions the drawer and Needs attention list. */
interface SessionsSource {
    /** Every session, in any order. The screens sort them. */
    fun sessions(): List<SessionSummary>
}

/** The data sources for the home screens, created once by the app module. */
data class HomeSources(
    val projects: ProjectsSource,
    val sessions: SessionsSource,
)
