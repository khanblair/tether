package app.tether.projectarchive

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSize
import app.tether.design.tokens.TetherSpace

/**
 * The Export project sheet. The Export button asks the route for a destination with
 * [onChooseDestination], passing the suggested file name. [onExported] runs once when the ZIP is
 * saved, so the route can close the sheet.
 */
@Composable
fun ExportProjectSheet(
    state: ExportProjectUiState,
    onIncludeIgnoredChange: (Boolean) -> Unit,
    onChooseDestination: (String) -> Unit,
    onTryAgain: () -> Unit,
    onExported: () -> Unit,
    onDismiss: () -> Unit,
) {
    LaunchedEffect(state.phase) {
        if (state.phase == ExportPhase.Done) onExported()
    }
    TetherSheet(title = stringResource(R.string.archive_export_title), onDismiss = onDismiss) {
        ExportProjectBody(state, onIncludeIgnoredChange, onChooseDestination, onTryAgain)
    }
}

/** The sheet content without the sheet, so previews can show it. */
@Composable
internal fun ExportProjectBody(
    state: ExportProjectUiState,
    onIncludeIgnoredChange: (Boolean) -> Unit,
    onChooseDestination: (String) -> Unit,
    onTryAgain: () -> Unit,
) {
    Column(
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        modifier = Modifier.fillMaxWidth().padding(horizontal = TetherSpace.space4),
    ) {
        Text(
            text = state.projectName,
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
        Text(
            text = stringResource(R.string.archive_export_explain),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        IncludeIgnoredRow(
            checked = state.includeIgnored,
            enabled = state.phase == ExportPhase.Editing,
            onChange = onIncludeIgnoredChange,
        )
        Text(
            text = stringResource(R.string.archive_include_ignored_help),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text3,
        )
        ExportActions(state, onChooseDestination, onTryAgain)
    }
}

/** The one action that matches the phase: Export, progress, Try again, or the done line. */
@Composable
private fun ExportActions(
    state: ExportProjectUiState,
    onChooseDestination: (String) -> Unit,
    onTryAgain: () -> Unit,
) {
    when (state.phase) {
        ExportPhase.Editing ->
            PrimaryButton(
                label = stringResource(R.string.archive_export_button),
                onClick = { onChooseDestination(state.suggestedName) },
                modifier = Modifier.fillMaxWidth(),
            )
        ExportPhase.Exporting -> ProgressWithText(text = stringResource(R.string.archive_exporting))
        ExportPhase.Failed ->
            state.reason?.let { reason ->
                InlineBanner(
                    text = stringResource(reason.messageRes()),
                    tone = BannerTone.Danger,
                    actionLabel = stringResource(R.string.archive_try_again),
                    onAction = onTryAgain,
                )
            }
        ExportPhase.Done ->
            Text(
                text = stringResource(R.string.archive_export_done),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
    }
}

/** The "Include ignored files" switch. The whole row is the touch target, at least 48 dp tall. */
@Composable
private fun IncludeIgnoredRow(
    checked: Boolean,
    enabled: Boolean,
    onChange: (Boolean) -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .heightIn(min = TetherSize.touchMin)
                .toggleable(
                    value = checked,
                    enabled = enabled,
                    role = Role.Switch,
                    onValueChange = onChange,
                ),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = stringResource(R.string.archive_include_ignored),
            style = MaterialTheme.typography.bodyLarge,
            color = if (enabled) TetherTheme.colors.text else TetherTheme.colors.textDisabled,
            modifier = Modifier.weight(1f),
        )
        Switch(checked = checked, onCheckedChange = null, enabled = enabled)
    }
}
