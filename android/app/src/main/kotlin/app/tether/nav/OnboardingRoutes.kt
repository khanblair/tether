package app.tether.nav

import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.R
import app.tether.di.AppModule
import app.tether.onboarding.AddKeyViewModel
import app.tether.onboarding.OnboardingExit
import app.tether.onboarding.OnboardingScreen
import app.tether.onboarding.OnboardingViewModel
import app.tether.settings.SettingsStore
import app.tether.shell.PlaceholderScreen

/** The GitHub sign-in screen, reached from the open-project step. Not built yet. */
const val ONBOARDING_GITHUB_ROUTE = "onboarding/github"

/** The zip import screen, reached from the open-project step. Not built yet. */
const val ONBOARDING_ARCHIVE_ROUTE = "onboarding/archive"

/**
 * The first screen the app shows. The first-run flow until it is finished or skipped once, then
 * Home. Pass the result as the nav host's start destination.
 */
fun startRoute(settings: SettingsStore): String =
    if (settings.onboarded()) Route.Home.path else Route.Onboarding.path

/** The first-run flow and the two sources it names but cannot open yet. */
internal fun NavGraphBuilder.onboardingRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(Route.Onboarding.path) {
        val settings = settingsDepsOf(LocalContext.current).store
        val onboarding: OnboardingViewModel =
            viewModel(factory = OnboardingViewModel.factory(settings))
        val addKey: AddKeyViewModel =
            viewModel(
                factory = AddKeyViewModel.factory(module.providerGateway, onboarding::next),
            )
        OnboardingScreen(
            onboarding = onboarding,
            addKey = addKey,
            onExit = nav::leaveOnboarding,
            onOpenGitHub = { nav.open(ONBOARDING_GITHUB_ROUTE) },
            onOpenArchive = { nav.open(ONBOARDING_ARCHIVE_ROUTE) },
        )
    }
    composable(ONBOARDING_ARCHIVE_ROUTE) {
        PlaceholderScreen(
            title = stringResource(R.string.onboarding_archive_title),
            explanation = stringResource(R.string.onboarding_archive_body),
            onBack = nav::back,
        )
    }
}

/** Ends the flow: Home becomes the only screen, and a new project opens above it. */
internal fun TetherNav.leaveOnboarding(exit: OnboardingExit) {
    openHomeAsRoot()
    if (exit == OnboardingExit.NewProject) go(Route.NewProject)
}
