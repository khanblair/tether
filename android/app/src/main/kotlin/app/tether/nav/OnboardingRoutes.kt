package app.tether.nav

import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.onboarding.AddKeyViewModel
import app.tether.onboarding.OnboardingExit
import app.tether.onboarding.OnboardingScreen
import app.tether.onboarding.OnboardingViewModel
import app.tether.settings.SettingsStore
import app.tether.workspace.WorkspaceViewModel
import app.tether.workspace.suggestedWorkspacePath

/**
 * The first screen the app shows. The first-run flow until it is finished or skipped once, then
 * Home. Pass the result as the nav host's start destination.
 */
fun startRoute(settings: SettingsStore): String =
    if (settings.onboarded()) Route.Home.path else Route.Onboarding.path

/** The first-run flow. */
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
        val settingsDeps = settingsDepsOf(LocalContext.current)
        val workspace: WorkspaceViewModel =
            viewModel(
                factory =
                    WorkspaceViewModel.factory(
                        gateway = module.workspaceGateway,
                        sharedFolderAllowed = module.workspaceCapabilities.sharedFolderAllowed,
                        initialAllFilesAccessGranted =
                            module.workspaceCapabilities.allFilesAccessGranted(
                                LocalContext.current,
                            ),
                        suggestedPath = suggestedWorkspacePath(),
                        paths = settingsDeps.storage::paths,
                    ),
            )
        OnboardingScreen(
            onboarding = onboarding,
            addKey = addKey,
            workspace = workspace,
            workspaceCapabilities = module.workspaceCapabilities,
            onExit = nav::leaveOnboarding,
        )
    }
}

/** Ends the flow: Home becomes the only screen, and the chosen project screen opens above it. */
internal fun TetherNav.leaveOnboarding(exit: OnboardingExit) {
    openHomeAsRoot()
    when (exit) {
        OnboardingExit.Home -> Unit
        OnboardingExit.NewProject -> go(Route.NewProject)
        OnboardingExit.Clone -> go(Route.CloneProject)
        OnboardingExit.Import -> go(Route.ImportProject)
    }
}
