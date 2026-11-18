package app.tether.nav

import androidx.annotation.StringRes
import app.tether.R

/** Every route in the app. Screens fill in as phases ship; see docs/build-plan.md. */
enum class Route(
    val path: String,
    @StringRes val title: Int,
) {
    Onboarding("onboarding", R.string.route_onboarding_title),
    Projects("projects", R.string.route_projects_title),
    ProjectHome("project-home", R.string.route_project_home_title),
    Session("session", R.string.route_session_title),
    Diff("diff", R.string.route_diff_title),
    Files("files", R.string.route_files_title),
    Runs("runs", R.string.route_runs_title),
    Linux("linux", R.string.route_linux_title),
    Settings("settings", R.string.route_settings_title),
    Cost("cost", R.string.route_cost_title),
}
