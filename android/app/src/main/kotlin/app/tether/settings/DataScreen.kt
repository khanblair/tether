package app.tether.settings

import android.text.format.Formatter
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherTextButton
import app.tether.workspace.WorkspaceDataPaths

/**
 * Storage by kind, and the update cache with its one clear action. Nothing here deletes projects.
 */
@Composable
fun DataScreen(
    viewModel: DataViewModel,
    onBack: () -> Unit,
    onOpenWorkspace: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val usage = state.usage
    val paths = state.paths

    val unknown = stringResource(R.string.settings_area_size_unknown)

    fun sizeOf(bytes: Long?): String =
        bytes?.let { Formatter.formatShortFileSize(context, it) } ?: unknown
    SettingsPage(title = stringResource(R.string.settings_area_data_title), onBack = onBack) {
        SectionTitle(text = stringResource(R.string.workspace_data_title))
        if (usage != null && paths != null) {
            DataLocations(
                paths = paths,
                usage = usage,
                sizeOf = ::sizeOf,
                onOpenWorkspace = onOpenWorkspace,
            )
        }
        SectionTitle(text = stringResource(R.string.settings_area_data_cache))
        if (state.cacheFailed) {
            InlineBanner(
                text = stringResource(R.string.settings_area_cache_failed),
                tone = BannerTone.Danger,
            )
        }
        ListGroup {
            ListRow(
                title = stringResource(R.string.settings_area_update_cache),
                subtitle = sizeOf(usage?.updateCache),
                trailing = {
                    TetherTextButton(
                        label = stringResource(R.string.settings_area_clear_update_cache),
                        onClick = viewModel::clearUpdateCache,
                    )
                },
            )
        }
    }
}

@Composable
private fun DataLocations(
    paths: WorkspaceDataPaths,
    usage: StorageUsage,
    sizeOf: (Long?) -> String,
    onOpenWorkspace: () -> Unit,
) {
    ListGroup {
        dataLocations(paths, usage).forEach { location ->
            ListRow(
                title = dataLocationTitle(location.kind),
                subtitle = location.path,
                trailing = { SizeText(sizeOf(location.size)) },
                onClick =
                    if (location.kind == DataLocationKind.Projects) {
                        onOpenWorkspace
                    } else {
                        null
                    },
            )
        }
    }
}

@Composable
private fun dataLocationTitle(kind: DataLocationKind): String =
    stringResource(
        when (kind) {
            DataLocationKind.Projects -> R.string.workspace_data_projects
            DataLocationKind.Database -> R.string.workspace_data_database
            DataLocationKind.Secrets -> R.string.workspace_data_secrets
            DataLocationKind.Checkpoints -> R.string.workspace_data_checkpoints
            DataLocationKind.Memory -> R.string.workspace_data_memory
            DataLocationKind.Skills -> R.string.workspace_data_skills
            DataLocationKind.Vault -> R.string.workspace_data_vault
            DataLocationKind.CrashReport -> R.string.workspace_data_crash_report
        },
    )

/** A size shown at the end of a row. The row's title carries the name of the item. */
@Composable
private fun SizeText(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text2,
    )
}
