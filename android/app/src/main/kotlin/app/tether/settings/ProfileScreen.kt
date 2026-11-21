package app.tether.settings

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SectionTitle

/** The local profile: one display name. Nothing about it leaves the phone. */
@Composable
fun ProfileScreen(
    viewModel: ProfileViewModel,
    onBack: () -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    SettingsPage(title = stringResource(R.string.settings_area_profile_title), onBack = onBack) {
        SectionTitle(text = stringResource(R.string.settings_area_profile_name))
        OutlinedTextField(
            value = state.name,
            onValueChange = viewModel::onNameChange,
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            label = { Text(text = stringResource(R.string.settings_area_profile_name)) },
            supportingText = {
                Text(
                    text = stringResource(R.string.settings_area_profile_note),
                    style = MaterialTheme.typography.bodySmall,
                    color = TetherTheme.colors.text3,
                )
            },
        )
        PrimaryButton(
            label = stringResource(R.string.settings_area_save),
            onClick = viewModel::save,
            enabled = state.canSave,
        )
    }
}
