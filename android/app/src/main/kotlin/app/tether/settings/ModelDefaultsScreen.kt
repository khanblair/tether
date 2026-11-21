package app.tether.settings

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.providers.providerNameRes

/** The app default model, the per-project models, and the two models the app cannot use yet. */
@Composable
fun ModelDefaultsScreen(
    viewModel: ModelDefaultsViewModel,
    onBack: () -> Unit,
    onOpenProviders: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    SettingsPage(title = stringResource(R.string.settings_area_models_title), onBack = onBack) {
        if (state.failed) {
            InlineBanner(
                text = stringResource(R.string.settings_area_models_failed),
                tone = BannerTone.Danger,
            )
        }
        SectionTitle(text = stringResource(R.string.settings_area_models_defaults))
        ListGroup {
            ListRow(
                title = stringResource(R.string.settings_area_default_model),
                subtitle =
                    state.appDefault?.modelId
                        ?: stringResource(R.string.settings_area_default_model_unset),
                trailing = { ChevronTrailing() },
                onClick = { viewModel.open(ModelTarget.App) },
            )
            NotAvailableRow(
                title = stringResource(R.string.settings_area_reviewer_model),
                explanation = stringResource(R.string.settings_area_reviewer_model_na),
            )
            NotAvailableRow(
                title = stringResource(R.string.settings_area_summary_model),
                explanation = stringResource(R.string.settings_area_summary_model_na),
            )
        }
        if (state.projects.isNotEmpty()) {
            SectionTitle(text = stringResource(R.string.settings_area_models_per_project))
            ListGroup {
                state.projects.forEach { row ->
                    ListRow(
                        title = row.name,
                        subtitle =
                            row.model?.modelId
                                ?: stringResource(R.string.settings_area_uses_app_default),
                        trailing = { ChevronTrailing() },
                        onClick = { viewModel.open(ModelTarget.Project(row.id, row.name)) },
                    )
                }
            }
        }
    }
    state.picking?.let { target ->
        ModelPickSheet(
            target = target,
            state = state,
            onPick = viewModel::choose,
            onOpenProviders = onOpenProviders,
            onDismiss = viewModel::close,
        )
    }
}

/** The picker: the saved models of every provider with a key, grouped by provider. */
@Composable
private fun ModelPickSheet(
    target: ModelTarget,
    state: ModelDefaultsUiState,
    onPick: (DefaultModel?) -> Unit,
    onOpenProviders: () -> Unit,
    onDismiss: () -> Unit,
) {
    val current =
        when (target) {
            ModelTarget.App -> state.appDefault
            is ModelTarget.Project -> state.projects.firstOrNull { it.id == target.id }?.model
        }
    TetherSheet(
        title = stringResource(R.string.settings_area_default_model),
        onDismiss = onDismiss,
    ) {
        if (state.providers.isEmpty()) {
            Text(
                text = stringResource(R.string.settings_area_models_none),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
                modifier = Modifier.fillMaxWidth(),
            )
            TetherTextButton(
                label = stringResource(R.string.settings_area_open_providers),
                onClick = onOpenProviders,
            )
        }
        if (target is ModelTarget.Project) {
            ListGroup {
                ListRow(
                    title = stringResource(R.string.settings_area_uses_app_default),
                    trailing = selectedTrailing(current == null),
                    onClick = { onPick(null) },
                )
            }
        }
        state.providers.forEach { group -> ProviderGroup(group, current, onPick) }
    }
}

@Composable
private fun ProviderGroup(
    group: ProviderModels,
    current: DefaultModel?,
    onPick: (DefaultModel?) -> Unit,
) {
    SectionTitle(text = stringResource(providerNameRes(group.provider)))
    ListGroup {
        group.models.forEach { model ->
            val selected = current?.provider == model.provider && current.modelId == model.id
            ListRow(
                title = model.displayName,
                subtitle = model.id,
                trailing = selectedTrailing(selected),
                onClick = { onPick(DefaultModel(model.provider, model.id)) },
            )
        }
    }
}
