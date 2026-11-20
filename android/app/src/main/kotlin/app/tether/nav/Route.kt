package app.tether.nav

import androidx.annotation.StringRes
import app.tether.R
import app.tether.core.generated.ProviderView

/** Every route in the app. Screens fill in as phases ship; see docs/build-plan.md. */
enum class Route(
    val path: String,
    @StringRes val title: Int,
) {
    Home("home", R.string.route_home_title),
    Onboarding("onboarding", R.string.route_onboarding_title),
    Projects("projects", R.string.route_projects_title),
    Attention("attention", R.string.route_attention_title),
    ProjectHome("project-home", R.string.route_project_home_title),
    Session("session", R.string.route_session_title),
    NewSession("new-session", R.string.route_new_session_title),
    Skills("skills", R.string.route_skills_title),
    CloneProject("clone-project", R.string.route_clone_title),
    NewProject("new-project", R.string.route_new_project_title),
    ImportProject("import-project", R.string.route_import_title),
    Diff("diff", R.string.route_diff_title),
    Files("files", R.string.route_files_title),
    Runs("runs", R.string.route_runs_title),
    Linux("linux", R.string.route_linux_title),
    Settings("settings", R.string.route_settings_title),
    Providers("providers", R.string.providers_title),
    About("about", R.string.about_title),
    Cost("cost", R.string.route_cost_title),
}

/** Routes whose path has arguments. They are not entries of [Route]. */
object Routes {
    const val PROVIDER_DETAIL = "providers/{provider}"
}

/** The detail path for [provider], such as "providers/ANTHROPIC". */
fun providerDetailPath(provider: ProviderView) = "providers/${provider.name}"
