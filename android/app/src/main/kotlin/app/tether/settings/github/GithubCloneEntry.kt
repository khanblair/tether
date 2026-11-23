package app.tether.settings.github

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import app.tether.home.NewProjectChoice

/**
 * The entry for the New project sheet's Clone choice. It opens [CloneScreen] for
 * [NewProjectChoice.Clone] and draws nothing for the other choices. [onCloned] receives the new
 * project ID after the clone succeeds.
 */
@Composable
fun GithubCloneEntry(
    choice: NewProjectChoice,
    gateway: GithubGateway,
    onCloned: (String) -> Unit,
    onSignIn: () -> Unit,
    onBack: () -> Unit,
) {
    if (choice != NewProjectChoice.Clone) return
    val viewModel: CloneViewModel = viewModel(factory = CloneViewModel.factory(gateway))
    val state by viewModel.state.collectAsStateWithLifecycle()
    CloneScreen(
        state = state,
        onBack = onBack,
        onUrlChange = viewModel::onUrlChange,
        onProjectNameChange = viewModel::onProjectNameChange,
        onHistoryChange = viewModel::onHistoryChange,
        onQueryChange = viewModel::onQueryChange,
        onPickRepo = viewModel::pickRepo,
        onLoadMore = viewModel::loadMore,
        onSignIn = onSignIn,
        onClone = viewModel::startClone,
        onCancel = viewModel::cancelClone,
        onCloned = { projectId ->
            viewModel.clearCloned()
            onCloned(projectId)
        },
    )
}
