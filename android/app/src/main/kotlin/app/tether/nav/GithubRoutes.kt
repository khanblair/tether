package app.tether.nav

import android.content.Intent
import android.net.Uri
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.di.AppModule
import app.tether.settings.github.CloneScreen
import app.tether.settings.github.CloneViewModel
import app.tether.settings.github.GitIdentityScreen
import app.tether.settings.github.GitIdentityViewModel
import app.tether.settings.github.GithubAccountScreen
import app.tether.settings.github.GithubAccountViewModel

/** The Git identity form, opened from the GitHub account screen. */
const val SETTINGS_GIT_IDENTITY_ROUTE = "settings/git-identity"

/** The GitHub account, the Git identity, and the clone screens. */
internal fun NavGraphBuilder.githubRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_GITHUB_ROUTE) { GithubAccountRoute(module, nav) }
    composable(ONBOARDING_GITHUB_ROUTE) { GithubAccountRoute(module, nav) }
    composable(SETTINGS_GIT_IDENTITY_ROUTE) {
        val viewModel: GitIdentityViewModel =
            viewModel(
                factory =
                    viewModelFactory {
                        initializer {
                            GitIdentityViewModel(
                                module.githubGateway,
                            )
                        }
                    },
            )
        val state by viewModel.state.collectAsStateWithLifecycle()
        GitIdentityScreen(
            state = state,
            onBack = nav::back,
            onNameChange = viewModel::onNameChange,
            onEmailChange = viewModel::onEmailChange,
            onSave = viewModel::save,
            onSaved = nav::back,
        )
    }
    composable(Route.CloneProject.path) {
        val viewModel: CloneViewModel =
            viewModel(factory = CloneViewModel.factory(module.githubGateway))
        val state by viewModel.state.collectAsStateWithLifecycle()
        CloneScreen(
            state = state,
            onBack = nav::back,
            onUrlChange = viewModel::onUrlChange,
            onProjectNameChange = viewModel::onProjectNameChange,
            onHistoryChange = viewModel::onHistoryChange,
            onQueryChange = viewModel::onQueryChange,
            onPickRepo = viewModel::pickRepo,
            onLoadMore = viewModel::loadMore,
            onSignIn = { nav.open(SETTINGS_GITHUB_ROUTE) },
            onClone = viewModel::startClone,
            onCancel = viewModel::cancelClone,
            onCloned = { projectId ->
                viewModel.clearCloned()
                nav.replace(projectHomePath(projectId), Route.CloneProject)
            },
        )
    }
}

@Composable
private fun GithubAccountRoute(
    module: AppModule,
    nav: TetherNav,
) {
    val context = LocalContext.current
    val viewModel: GithubAccountViewModel =
        viewModel(
            factory =
                viewModelFactory {
                    initializer {
                        GithubAccountViewModel(
                            module.githubGateway,
                        )
                    }
                },
        )
    val state by viewModel.state.collectAsStateWithLifecycle()
    GithubAccountScreen(
        state = state,
        onBack = nav::back,
        onSignInWithDevice = viewModel::startDeviceFlow,
        onStopPolling = viewModel::stopPolling,
        onOpenUrl = { url -> openHttpsUrl(context, url) },
        onTokenChange = viewModel::onTokenChange,
        onSignInWithToken = viewModel::signInWithToken,
        onSignOut = viewModel::signOut,
        onOpenIdentity = { nav.open(SETTINGS_GIT_IDENTITY_ROUTE) },
    )
}

/** Opens [url] in the browser when it is an https address. The device flow gives GitHub's page. */
internal fun openHttpsUrl(
    context: android.content.Context,
    url: String,
) {
    val uri = Uri.parse(url)
    if (uri.scheme != "https") return
    context.startActivity(Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
}
