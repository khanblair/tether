package app.tether.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.PrimaryButton
import app.tether.design.components.TetherSheet
import app.tether.design.tokens.TetherSpace

/** The session menu. Rename and Rewind are the only actions the core supports today. */
@Composable
fun SessionMenuSheet(
    onRename: () -> Unit,
    onRewind: () -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.session_menu_title), onDismiss = onDismiss) {
        ListGroup(modifier = Modifier.padding(horizontal = TetherSpace.space4)) {
            ListRow(
                title = stringResource(R.string.session_menu_rename),
                onClick = onRename,
            )
            ListRow(
                title = stringResource(R.string.session_menu_rewind),
                subtitle = stringResource(R.string.session_menu_rewind_sub),
                onClick = onRewind,
            )
        }
    }
}

/**
 * The rename sheet: one field. The title is trimmed before it is saved, and a blank title cannot
 * be saved.
 */
@Composable
fun RenameSheet(
    state: RenameState,
    onTitleChange: (String) -> Unit,
    onSave: () -> Unit,
    onDismiss: () -> Unit,
) {
    TetherSheet(title = stringResource(R.string.session_rename_title), onDismiss = onDismiss) {
        Column(
            modifier = Modifier.padding(horizontal = TetherSpace.space4),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            OutlinedTextField(
                value = state.title,
                onValueChange = onTitleChange,
                singleLine = true,
                label = { Text(stringResource(R.string.session_rename_label)) },
                modifier = Modifier.fillMaxWidth(),
            )
            if (state.failure != null) {
                InlineBanner(
                    text = stringResource(R.string.session_error_settings),
                    tone = BannerTone.Danger,
                )
            }
            PrimaryButton(
                label = stringResource(R.string.session_rename_save),
                onClick = onSave,
                modifier = Modifier.fillMaxWidth(),
                enabled = state.canSave,
                loading = state.saving,
            )
        }
    }
}
