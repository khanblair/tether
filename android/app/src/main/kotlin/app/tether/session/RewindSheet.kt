package app.tether.session

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherSheet
import app.tether.design.components.TetherTextButton
import app.tether.design.tokens.TetherSpace
import app.tether.home.relativeTime
import app.tether.home.text

/**
 * The rewind sheet: checkpoints newest first, the three restore scopes, and a button that names
 * what it restores. A files scope asks again before it runs.
 */
@Composable
fun RewindSheet(
    state: RewindState,
    now: Long,
    onSelect: (String) -> Unit,
    onScope: (RewindScope) -> Unit,
    onRewind: () -> Unit,
    onConfirm: () -> Unit,
    onDismissConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.session_rewind_title), onDismiss = onDismiss) {
        Column(
            modifier =
                Modifier
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            Checkpoints(state, now, onSelect)
            SectionTitle(text = stringResource(R.string.session_rewind_restore))
            ScopeChips(selected = state.scope, enabled = !state.applying, onScope = onScope)
            RewindFailure(state.failure)
            PrimaryButton(
                label = stringResource(scopeButtonRes(state.scope)),
                onClick = onRewind,
                modifier = Modifier.fillMaxWidth(),
                enabled = state.selected != null && !state.applying,
                loading = state.applying,
            )
            Text(
                text = restoreHint(state),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text2,
            )
        }
    }
    if (state.confirming) {
        FilesConfirmation(onConfirm = onConfirm, onDismiss = onDismissConfirm)
    }
}

@Composable
private fun Checkpoints(
    state: RewindState,
    now: Long,
    onSelect: (String) -> Unit,
) {
    when {
        state.loading ->
            Text(
                text = stringResource(R.string.session_rewind_loading),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
        state.checkpoints.isEmpty() ->
            Text(
                text = stringResource(R.string.session_rewind_none),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
        else ->
            ListGroup {
                state.checkpoints.forEach { checkpoint ->
                    CheckpointRow(
                        checkpoint = checkpoint,
                        now = now,
                        selected = checkpoint.id == state.selected,
                        enabled = !state.applying,
                        onClick = { onSelect(checkpoint.id) },
                    )
                }
            }
    }
}

/** One checkpoint: when it was saved, how many files it holds, and their paths. */
@Composable
private fun CheckpointRow(
    checkpoint: CheckpointItem,
    now: Long,
    selected: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
) {
    val count = checkpoint.paths.size
    val ago = relativeTime(now, checkpoint.createdAtMs).text()
    val files = pluralStringResource(R.plurals.session_rewind_files, count, count)
    ListRow(
        title = "$ago · $files",
        subtitle = checkpoint.paths.joinToString(separator = ", "),
        trailing =
            if (selected) {
                {
                    TetherIcon(
                        icon = TetherIcons.CircleCheck,
                        contentDescription = stringResource(R.string.session_selected),
                        tint = TetherTheme.colors.accent,
                    )
                }
            } else {
                null
            },
        onClick = if (enabled) onClick else null,
        enabled = enabled,
    )
}

/** The three restore scopes. A selected chip has a fill and a check, not color alone. */
@Composable
private fun ScopeChips(
    selected: RewindScope,
    enabled: Boolean,
    onScope: (RewindScope) -> Unit,
) {
    Row(
        modifier = Modifier.horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(TetherSpace.space2),
    ) {
        RewindScope.entries.forEach { scope ->
            FilterChip(
                selected = scope == selected,
                onClick = { onScope(scope) },
                enabled = enabled,
                label = {
                    Text(
                        text = stringResource(scopeChipRes(scope)),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                },
            )
        }
    }
}

@Composable
private fun RewindFailure(failure: SessionFailure?) {
    if (failure == null) return
    val text =
        when (failure) {
            SessionFailure.CORRUPT_SNAPSHOT -> stringResource(R.string.session_error_corrupt)
            SessionFailure.INVALID_PATH -> stringResource(R.string.session_error_invalid_path)
            else -> stringResource(R.string.session_error_rewind)
        }
    InlineBanner(text = text, tone = BannerTone.Danger)
}

@Composable
private fun FilesConfirmation(
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.session_rewind_confirm_title)) },
        text = { Text(stringResource(R.string.session_rewind_confirm_body)) },
        confirmButton = {
            TextButton(onClick = onConfirm) {
                Text(stringResource(R.string.session_rewind_confirm_action))
            }
        },
        dismissButton = {
            TetherTextButton(
                label = stringResource(R.string.session_rewind_cancel),
                onClick = onDismiss,
            )
        },
    )
}

/** The button text for a scope. It says exactly what the rewind restores. */
internal fun scopeButtonRes(scope: RewindScope): Int =
    when (scope) {
        RewindScope.FILES_AND_CONVERSATION -> R.string.session_rewind_button_both
        RewindScope.FILES -> R.string.session_rewind_button_files
        RewindScope.CONVERSATION -> R.string.session_rewind_button_conversation
    }

private fun scopeChipRes(scope: RewindScope): Int =
    when (scope) {
        RewindScope.FILES_AND_CONVERSATION -> R.string.session_rewind_chip_both
        RewindScope.FILES -> R.string.session_rewind_chip_files
        RewindScope.CONVERSATION -> R.string.session_rewind_chip_conversation
    }

/** One line under the button: how many files the rewind touches, or what it removes. */
@Composable
private fun restoreHint(state: RewindState): String {
    val files =
        state.checkpoints
            .firstOrNull { it.id == state.selected }
            ?.paths
            ?.size ?: 0
    val fileText = pluralStringResource(R.plurals.session_rewind_hint_files, files, files)
    return when (state.scope) {
        RewindScope.FILES_AND_CONVERSATION ->
            stringResource(R.string.session_rewind_hint_both, fileText)
        RewindScope.FILES -> fileText
        RewindScope.CONVERSATION -> stringResource(R.string.session_rewind_hint_conversation)
    }
}
