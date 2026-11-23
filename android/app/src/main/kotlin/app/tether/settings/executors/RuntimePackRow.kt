package app.tether.settings.executors

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.session.tools.ExecutorBadge
import app.tether.session.tools.RunExecutor

/**
 * One sandbox runtime pack: its name, its state, and the one action that state allows. Install
 * and Try again both call [onInstall]. Cancel stops the screen from waiting for a download.
 * [source] is the host the pack downloads from, shown before the user installs it.
 */
@Composable
fun RuntimePackRow(
    pack: PackRow,
    onInstall: () -> Unit,
    onCancel: () -> Unit,
    modifier: Modifier = Modifier,
    source: String? = null,
) {
    Column(
        modifier =
            modifier
                .fillMaxWidth()
                .padding(horizontal = TetherSpace.space4, vertical = TetherSpace.space3),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
        ) {
            Text(
                text = pack.id,
                style = MaterialTheme.typography.bodyLarge,
                color = TetherTheme.colors.text,
                modifier = Modifier.weight(1f, fill = false),
            )
            ExecutorBadge(executor = RunExecutor.Sandbox)
        }
        PackStateBody(pack = pack, source = source, onInstall = onInstall, onCancel = onCancel)
    }
}

/** The state text and its action, for each pack state. */
@Composable
private fun PackStateBody(
    pack: PackRow,
    source: String?,
    onInstall: () -> Unit,
    onCancel: () -> Unit,
) {
    val colors = TetherTheme.colors
    val body = MaterialTheme.typography.bodySmall
    when (val state = pack.state) {
        PackState.NotInstalled -> {
            Text(
                text = stringResource(R.string.executors_pack_not_installed),
                style = body,
                color = colors.text2,
            )
            if (pack.sizeBytes > 0L) {
                Text(
                    text =
                        stringResource(
                            R.string.executors_pack_download_size,
                            sizeText(pack.sizeBytes),
                        ),
                    style = body,
                    color = colors.text3,
                )
            }
            SourceLine(source)
            TetherTextButton(
                label = stringResource(R.string.executors_install),
                onClick = onInstall,
            )
        }
        is PackState.Downloading -> {
            Text(
                text = downloadingText(state.downloaded, state.total),
                style = body,
                color = colors.text2,
            )
            if (state.total > 0L) {
                LinearProgressIndicator(
                    progress = { downloadFraction(state.downloaded, state.total) },
                    modifier = Modifier.fillMaxWidth(),
                )
            } else {
                LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            }
            TetherTextButton(label = stringResource(R.string.executors_cancel), onClick = onCancel)
        }
        PackState.Verifying -> {
            Text(
                text = stringResource(R.string.executors_pack_verifying),
                style = body,
                color = colors.text2,
            )
            LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
        }
        PackState.Detached ->
            Text(
                text = stringResource(R.string.executors_cancel_note),
                style = body,
                color = colors.text3,
            )
        is PackState.Ready ->
            Text(
                text = stringResource(R.string.executors_pack_ready, state.version),
                style = body,
                color = colors.text2,
            )
        is PackState.Failed -> {
            Text(
                text = stringResource(R.string.executors_pack_failed),
                style = body,
                color = colors.danger,
            )
            Text(text = state.failure.text(), style = body, color = colors.text2)
            SourceLine(source)
            TetherTextButton(
                label = stringResource(R.string.executors_try_again),
                onClick = onInstall,
            )
        }
        PackState.NotAvailable -> {
            Text(
                text = stringResource(R.string.executors_not_available),
                style = body,
                color = colors.text2,
            )
            Text(
                text = stringResource(R.string.executors_na_config),
                style = body,
                color = colors.text3,
            )
        }
    }
}

/** The host a download comes from, in monospace. Nothing is drawn when the host is unknown. */
@Composable
internal fun SourceLine(source: String?) {
    if (source == null) return
    Text(
        text = stringResource(R.string.executors_source, source),
        style = TetherTheme.monoSmall,
        color = TetherTheme.colors.text3,
    )
}

/** A size in megabytes, such as "620 MB". */
@Composable
internal fun sizeText(bytes: Long): String =
    stringResource(R.string.executors_size_mb, megabytes(bytes))

/** "Downloading. 12 MB of 40 MB", or just "Downloading" while the size is not known. */
@Composable
private fun downloadingText(
    downloaded: Long,
    total: Long,
): String =
    if (total > 0L) {
        stringResource(
            R.string.executors_pack_downloading,
            sizeText(downloaded),
            sizeText(total),
        )
    } else {
        stringResource(R.string.executors_pack_downloading_unknown)
    }
