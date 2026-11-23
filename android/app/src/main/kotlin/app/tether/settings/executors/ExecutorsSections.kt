package app.tether.settings.executors

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace

/** The CI readiness sentence, and a link to the runs list when a project is open. */
@Composable
internal fun CiSection(
    state: CiState,
    onRetry: () -> Unit,
    onOpenCiRuns: () -> Unit,
) {
    ListGroup {
        when (state) {
            CiState.NoProject -> SectionNote(text = stringResource(R.string.executors_no_project))
            CiState.Loading -> LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            is CiState.Status -> {
                SectionNote(text = state.readiness.text())
                TetherTextButton(
                    label = stringResource(R.string.executors_ci_view_runs),
                    onClick = onOpenCiRuns,
                    modifier = Modifier.padding(horizontal = TetherSpace.space2),
                )
            }
            is CiState.Failed -> {
                SectionNote(text = state.failure.text())
                TetherTextButton(
                    label = stringResource(R.string.executors_try_again),
                    onClick = onRetry,
                    modifier = Modifier.padding(horizontal = TetherSpace.space2),
                )
            }
        }
    }
}

/** The network switch for the open project, or the reason it cannot be shown. */
@Composable
internal fun NetworkSection(
    state: NetworkState,
    onSet: (Boolean) -> Unit,
    onRetry: () -> Unit,
) {
    ListGroup {
        when (state) {
            NetworkState.NoProject ->
                SectionNote(
                    text = stringResource(R.string.executors_no_project),
                )
            NetworkState.Loading -> LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            is NetworkState.Loaded -> LoadedNetwork(state = state, onSet = onSet)
            is NetworkState.LoadFailed ->
                InlineBanner(
                    text = state.failure.text(),
                    tone = BannerTone.Danger,
                    actionLabel = stringResource(R.string.executors_try_again),
                    onAction = onRetry,
                )
        }
    }
}

/** The switch row. The whole row is the toggle, and the switch is only drawn. */
@Composable
private fun LoadedNetwork(
    state: NetworkState.Loaded,
    onSet: (Boolean) -> Unit,
) {
    ListRow(
        title = stringResource(R.string.executors_network_title),
        subtitle = stringResource(R.string.executors_network_body),
        trailing = {
            Switch(
                checked = state.allowed,
                onCheckedChange = null,
                enabled = !state.saving,
            )
        },
        modifier =
            Modifier.toggleable(
                value = state.allowed,
                enabled = !state.saving,
                role = Role.Switch,
                onValueChange = onSet,
            ),
    )
    state.failure?.let {
        InlineBanner(
            text = stringResource(R.string.executors_network_failed),
            tone = BannerTone.Danger,
        )
    }
}

/** A sentence in a list group, with the same padding as a row. */
@Composable
private fun SectionNote(text: String) {
    Column(modifier = Modifier.padding(TetherSpace.space4)) {
        Text(
            text = text,
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
    }
}
