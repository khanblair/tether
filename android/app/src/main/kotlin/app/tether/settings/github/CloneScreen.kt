package app.tether.settings.github

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SegmentedControl
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.settings.SettingsPage

/**
 * The clone screen: a URL field, the signed-in repositories, the history choice, the project name,
 * and Clone. While a clone runs it shows the phase and bytes, and Cancel. A success calls
 * [onCloned] with the new project ID. A failure shows its reason with Try again.
 */
@Composable
fun CloneScreen(
    state: CloneUiState,
    onBack: () -> Unit,
    onUrlChange: (String) -> Unit,
    onProjectNameChange: (String) -> Unit,
    onHistoryChange: (CloneHistory) -> Unit,
    onQueryChange: (String) -> Unit,
    onPickRepo: (GithubRepo) -> Unit,
    onLoadMore: () -> Unit,
    onSignIn: () -> Unit,
    onClone: () -> Unit,
    onCancel: () -> Unit,
    onCloned: (String) -> Unit,
) {
    LaunchedEffect(state.clonedProjectId) {
        state.clonedProjectId?.let(onCloned)
    }
    val historyLabels = CloneHistory.entries.associateWith { stringResource(it.labelRes()) }
    SettingsPage(
        title = stringResource(R.string.github_clone_title),
        onBack = onBack,
    ) {
        OutlinedTextField(
            value = state.url,
            onValueChange = onUrlChange,
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            enabled = !state.cloning,
            label = { Text(stringResource(R.string.github_clone_url_label)) },
            placeholder = { Text(stringResource(R.string.github_clone_url_placeholder)) },
        )
        when (state.signedIn) {
            null -> Unit
            false ->
                Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space3)) {
                    Text(
                        text = stringResource(R.string.github_clone_sign_in_first),
                        style = MaterialTheme.typography.bodyMedium,
                        color = TetherTheme.colors.text2,
                    )
                    PrimaryButton(
                        label = stringResource(R.string.github_clone_sign_in_button),
                        onClick = onSignIn,
                    )
                }
            true ->
                CloneRepoList(
                    repos = state.repos,
                    query = state.query,
                    selectedUrl = state.url.trim(),
                    hasMore = state.reposHaveMore,
                    loading = state.reposLoading,
                    failure = state.reposFailure,
                    onQueryChange = onQueryChange,
                    onPick = onPickRepo,
                    onLoadMore = onLoadMore,
                )
        }
        OutlinedTextField(
            value = state.projectName,
            onValueChange = onProjectNameChange,
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            enabled = !state.cloning,
            label = { Text(stringResource(R.string.github_clone_name_label)) },
            supportingText = { Text(stringResource(R.string.github_clone_name_hint)) },
        )
        Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
            Text(
                text = stringResource(R.string.github_clone_history_label),
                style = MaterialTheme.typography.labelLarge,
                color = TetherTheme.colors.text2,
            )
            SegmentedControl(
                options = CloneHistory.entries,
                selected = state.history,
                onSelect = onHistoryChange,
                label = { historyLabels.getValue(it) },
                enabled = !state.cloning,
            )
            Text(
                text = stringResource(R.string.github_clone_history_hint),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
        if (state.cloning) {
            CloneProgressPanel(state.progress, onCancel)
        } else {
            CloneTarget(
                url = state.url.trim(),
                enabled = state.url.isNotBlank(),
                onClone = onClone,
            )
        }
        if (state.canceled) {
            InlineBanner(
                text = stringResource(R.string.github_clone_canceled),
                tone = BannerTone.Info,
            )
        }
        state.failure?.let { failure ->
            InlineBanner(
                text = stringResource(failure.messageRes()),
                tone = BannerTone.Danger,
                actionLabel = stringResource(R.string.github_clone_try_again),
                onAction = onClone,
            )
        }
    }
}

/** The progress of a running clone: the phase, the bytes, a bar, and Cancel. */
@Composable
private fun CloneProgressPanel(
    progress: CloneProgress?,
    onCancel: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
        Text(
            text = stringResource((progress?.phase ?: ClonePhaseStep.Unknown).labelRes()),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
        val total = progress?.total
        if (progress != null && total != null && total > 0) {
            LinearProgressIndicator(
                progress = { (progress.received.toFloat() / total.toFloat()).coerceIn(0f, 1f) },
                modifier = Modifier.fillMaxWidth(),
            )
            Text(
                text =
                    stringResource(
                        R.string.github_clone_received_total,
                        megabytesText(progress.received),
                        megabytesText(total),
                    ),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        } else {
            LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            progress?.let {
                Text(
                    text =
                        stringResource(
                            R.string.github_clone_received_only,
                            megabytesText(it.received),
                        ),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                )
            }
        }
        TetherTextButton(
            label = stringResource(R.string.github_clone_cancel),
            onClick = onCancel,
        )
    }
}
