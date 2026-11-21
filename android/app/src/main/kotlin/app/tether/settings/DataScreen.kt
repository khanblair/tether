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

/**
 * Storage by kind, and the update cache with its one clear action. Nothing here deletes projects.
 */
@Composable
fun DataScreen(
    viewModel: DataViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val usage = state.usage

    val unknown = stringResource(R.string.settings_area_size_unknown)

    fun sizeOf(bytes: Long?): String =
        bytes?.let { Formatter.formatShortFileSize(context, it) } ?: unknown
    SettingsPage(title = stringResource(R.string.settings_area_data_title), onBack = onBack) {
        SectionTitle(text = stringResource(R.string.settings_area_data_stored))
        ListGroup {
            ListRow(
                title = stringResource(R.string.settings_area_data_database),
                subtitle = stringResource(R.string.settings_area_data_database_body),
                trailing = { SizeText(sizeOf(usage?.database)) },
            )
            ListRow(
                title = stringResource(R.string.settings_area_data_workspaces),
                subtitle = stringResource(R.string.settings_area_data_workspaces_body),
                trailing = { SizeText(sizeOf(usage?.workspaces)) },
            )
            ListRow(
                title = stringResource(R.string.settings_area_data_blobs),
                subtitle = stringResource(R.string.settings_area_data_blobs_body),
                trailing = { SizeText(sizeOf(usage?.checkpointBlobs)) },
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

/** A size shown at the end of a row. The row's title carries the name of the item. */
@Composable
private fun SizeText(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text2,
    )
}
