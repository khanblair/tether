package app.tether.settings

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
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
import app.tether.session.SessionMode
import app.tether.session.descriptionRes
import app.tether.session.labelRes

/** The default permission mode, the per-project modes, and the rules the app cannot save yet. */
@Composable
fun PermissionsScreen(
    viewModel: PermissionsViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    SettingsPage(
        title = stringResource(R.string.settings_area_permissions_title),
        onBack = onBack,
    ) {
        if (state.failed) {
            InlineBanner(
                text = stringResource(R.string.settings_area_permissions_failed),
                tone = BannerTone.Danger,
            )
        }
        SectionTitle(text = stringResource(R.string.settings_area_permissions_defaults))
        ListGroup {
            ListRow(
                title = stringResource(R.string.settings_area_default_mode),
                subtitle = stringResource(state.defaultMode.labelRes()),
                trailing = { ChevronTrailing() },
                onClick = { viewModel.open(ModeTarget.App) },
            )
            NotAvailableRow(
                title = stringResource(R.string.settings_area_saved_rules),
                explanation = stringResource(R.string.settings_area_saved_rules_na),
            )
            NotAvailableRow(
                title = stringResource(R.string.settings_area_push_guard),
                explanation = stringResource(R.string.settings_area_push_guard_na),
            )
        }
        Text(
            text = stringResource(R.string.settings_area_bypass_default_note),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
        if (state.projects.isNotEmpty()) {
            SectionTitle(text = stringResource(R.string.settings_area_models_per_project))
            ListGroup {
                state.projects.forEach { row ->
                    ListRow(
                        title = row.name,
                        subtitle =
                            row.mode?.let { stringResource(it.labelRes()) }
                                ?: stringResource(R.string.settings_area_uses_app_default),
                        trailing = { ChevronTrailing() },
                        onClick = { viewModel.open(ModeTarget.Project(row.id, row.name)) },
                    )
                }
            }
        }
    }
    state.picking?.let { target ->
        val current =
            when (target) {
                ModeTarget.App -> state.defaultMode
                is ModeTarget.Project -> state.projects.firstOrNull { it.id == target.id }?.mode
            }
        ModePickSheet(
            target = target,
            current = current,
            onPick = viewModel::choose,
            onDismiss = viewModel::close,
        )
    }
}

/**
 * The picker: the permission modes except Bypass, with their descriptions, and for a project, the
 * app default.
 */
@Composable
private fun ModePickSheet(
    target: ModeTarget,
    current: SessionMode?,
    onPick: (SessionMode?) -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.session_mode_title), onDismiss = onDismiss) {
        if (target is ModeTarget.Project) {
            ListGroup {
                ListRow(
                    title = stringResource(R.string.settings_area_uses_app_default),
                    trailing = selectedTrailing(current == null),
                    onClick = { onPick(null) },
                )
            }
        }
        ListGroup {
            SessionMode.entries.filterNot { it == SessionMode.BYPASS }.forEach { mode ->
                ListRow(
                    title = stringResource(mode.labelRes()),
                    subtitle = stringResource(mode.descriptionRes()),
                    trailing = selectedTrailing(mode == current),
                    onClick = { onPick(mode) },
                )
            }
        }
    }
}
