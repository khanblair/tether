package app.tether.settings.github

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.PrimaryButton
import app.tether.design.components.SecretField
import app.tether.design.tokens.TetherSpace

/**
 * The personal access token alternative to the device flow: a masked field with Paste, and a
 * Sign in button. The token is never shown again once typed, so there is no reveal control.
 * [emptyToken] shows the refusal under the field.
 */
@Composable
fun GithubTokenForm(
    token: String,
    emptyToken: Boolean,
    busy: Boolean,
    onTokenChange: (String) -> Unit,
    onSignIn: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    Column(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
    ) {
        Text(
            text = stringResource(R.string.github_account_token_title),
            style = MaterialTheme.typography.titleSmall,
            color = TetherTheme.colors.text,
        )
        SecretField(
            value = token,
            onValueChange = onTokenChange,
            label = stringResource(R.string.github_account_token_label),
            revealed = false,
            onToggleReveal = null,
            revealLabel = stringResource(R.string.github_account_token_reveal),
            hideLabel = stringResource(R.string.github_account_token_hide),
            pasteLabel = stringResource(R.string.github_account_token_paste),
            placeholder = stringResource(R.string.github_account_token_placeholder),
            hint = stringResource(R.string.github_account_token_hint),
            error = if (emptyToken) stringResource(R.string.github_account_token_empty) else null,
            enabled = !busy,
            onPaste = { readClipboardText(context)?.let(onTokenChange) },
        )
        PrimaryButton(
            label = stringResource(R.string.github_account_token_sign_in),
            onClick = onSignIn,
            enabled = !busy,
            loading = busy,
        )
    }
}
