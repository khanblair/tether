package app.tether.settings.github

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.BannerTone
import app.tether.design.components.InlineBanner
import app.tether.design.components.PrimaryButton
import app.tether.design.tokens.TetherSpace
import app.tether.settings.SettingsPage

/**
 * The Git identity form: the author name and email that commits use. Signing in to GitHub fills
 * these in. [onSaved] runs once after the core stores the identity.
 */
@Composable
fun GitIdentityScreen(
    state: IdentityUiState,
    onBack: () -> Unit,
    onNameChange: (String) -> Unit,
    onEmailChange: (String) -> Unit,
    onSave: () -> Unit,
    onSaved: () -> Unit,
) {
    LaunchedEffect(state.saved) {
        if (state.saved) onSaved()
    }
    SettingsPage(
        title = stringResource(R.string.github_identity_title),
        onBack = onBack,
    ) {
        Text(
            text = stringResource(R.string.github_identity_intro),
            style = MaterialTheme.typography.bodyMedium,
            color = TetherTheme.colors.text2,
        )
        Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space3)) {
            OutlinedTextField(
                value = state.name,
                onValueChange = onNameChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text(stringResource(R.string.github_identity_name_label)) },
                placeholder = { Text(stringResource(R.string.github_identity_name_placeholder)) },
                isError = state.nameProblem != null,
                supportingText = {
                    state.nameProblem?.let { problem ->
                        Text(stringResource(problem.messageRes(isEmail = false)))
                    }
                },
            )
            OutlinedTextField(
                value = state.email,
                onValueChange = onEmailChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                label = { Text(stringResource(R.string.github_identity_email_label)) },
                placeholder = { Text(stringResource(R.string.github_identity_email_placeholder)) },
                isError = state.emailProblem != null,
                supportingText = {
                    state.emailProblem?.let { problem ->
                        Text(stringResource(problem.messageRes(isEmail = true)))
                    }
                },
            )
        }
        PrimaryButton(
            label = stringResource(R.string.github_identity_save),
            onClick = onSave,
            enabled = !state.saving,
            loading = state.saving,
        )
        if (state.saved) {
            InlineBanner(
                text = stringResource(R.string.github_identity_saved),
                tone = BannerTone.Success,
            )
        }
        state.failure?.let { failure ->
            InlineBanner(
                text = stringResource(failure.messageRes()),
                tone = BannerTone.Danger,
            )
        }
    }
}
