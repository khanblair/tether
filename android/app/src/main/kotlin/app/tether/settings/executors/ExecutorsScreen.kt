package app.tether.settings.executors

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.settings.SettingsPage

/**
 * The Where commands run screen: the sandbox packs, Linux mode, CI, and network access for the
 * open project. Each action is offered only when its state allows it.
 */
@Composable
fun ExecutorsScreen(
    viewModel: ExecutorsViewModel,
    onBack: () -> Unit,
    onOpenCiRuns: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val repair: (() -> Unit)? = if (state.linuxSetupInBuild) viewModel::repairLinux else null
    SettingsPage(title = stringResource(R.string.executors_title), onBack = onBack) {
        SectionTitle(text = stringResource(R.string.executors_section_sandbox))
        SandboxSection(
            state = state,
            onInstall = viewModel::installPack,
            onCancel = viewModel::cancelPackInstall,
            onRetry = viewModel::retryPacks,
        )
        SectionTitle(text = stringResource(R.string.executors_section_linux))
        ListGroup {
            LinuxSetupCard(
                state = state.linux,
                onInstall = viewModel::installLinux,
                onRepair = repair,
                source = state.linuxSource,
            )
        }
        SectionTitle(text = stringResource(R.string.executors_section_ci))
        CiSection(state = state.ci, onRetry = viewModel::retryCi, onOpenCiRuns = onOpenCiRuns)
        SectionTitle(text = stringResource(R.string.executors_section_network))
        NetworkSection(
            state = state.network,
            onSet = viewModel::setNetworkAllowed,
            onRetry = viewModel::retryNetwork,
        )
    }
}

/** The pack rows, or the reason the sandbox packs cannot be read or installed. */
@Composable
private fun SandboxSection(
    state: ExecutorsUiState,
    onInstall: (String) -> Unit,
    onCancel: (String) -> Unit,
    onRetry: () -> Unit,
) {
    val failure = state.packsFailure
    val rows = state.packRows()
    when {
        !state.packsLoaded -> LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
        failure == ExecutorsFailure.NotConfigured ->
            ListGroup {
                ListRow(
                    title = stringResource(R.string.executors_not_available),
                    subtitle = stringResource(R.string.executors_na_config),
                )
            }
        failure != null ->
            ListGroup {
                Column(modifier = Modifier.padding(TetherSpace.space4)) {
                    Text(
                        text = failure.text(),
                        style = MaterialTheme.typography.bodySmall,
                        color = TetherTheme.colors.text2,
                    )
                    TetherTextButton(
                        label = stringResource(R.string.executors_try_again),
                        onClick = onRetry,
                    )
                }
            }
        rows.isEmpty() ->
            ListGroup {
                Text(
                    text = stringResource(R.string.executors_packs_empty),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text2,
                    modifier = Modifier.padding(TetherSpace.space4),
                )
            }
        else ->
            ListGroup {
                rows.forEachIndexed { index, row ->
                    if (index > 0) HorizontalDivider(color = TetherTheme.colors.line)
                    RuntimePackRow(
                        pack = row,
                        onInstall = { onInstall(row.id) },
                        onCancel = { onCancel(row.id) },
                        source = state.packSource,
                    )
                }
            }
    }
}
