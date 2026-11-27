package app.tether.settings.github

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherTextButton
import app.tether.design.components.TextButtonTone
import app.tether.design.tokens.TetherBorder
import app.tether.design.tokens.TetherSpace

/**
 * The saved account: login, identity, token access, and Sign out. When no account is saved it shows
 * one sentence of what signing in does. Signing in is a separate control on the screen.
 */
@Composable
fun GitHubAccountCard(
    status: GithubStatus,
    onSignOut: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier = modifier.fillMaxWidth()) {
        if (status.signedIn) {
            val login = status.login ?: status.name.orEmpty()
            val access = status.accountAccess()
            Text(
                text = stringResource(R.string.github_account_signed_in_as, login),
                style = MaterialTheme.typography.titleSmall,
                color = TetherTheme.colors.text,
                modifier = Modifier.padding(bottom = TetherSpace.space2),
            )
            ListGroup {
                status.name?.takeIf { it.isNotBlank() }?.let { name ->
                    ListRow(title = stringResource(R.string.github_account_name), subtitle = name)
                    HorizontalDivider(
                        thickness = TetherBorder.hairline,
                        color = TetherTheme.colors.line,
                    )
                }
                status.email?.takeIf { it.isNotBlank() }?.let { email ->
                    ListRow(title = stringResource(R.string.github_account_email), subtitle = email)
                    HorizontalDivider(
                        thickness = TetherBorder.hairline,
                        color = TetherTheme.colors.line,
                    )
                }
                ListRow(
                    title = stringResource(R.string.github_account_token_type),
                    subtitle =
                        access.tokenKind?.let { stringResource(it.labelRes()) }
                            ?: stringResource(R.string.github_account_token_kind_unknown),
                )
                HorizontalDivider(
                    thickness = TetherBorder.hairline,
                    color = TetherTheme.colors.line,
                )
                ListRow(
                    title = stringResource(R.string.github_account_repositories),
                    subtitle = stringResource(access.repositoryAccess.labelRes()),
                )
                if (access.scopePresentation == GithubScopePresentation.ClassicScopes) {
                    HorizontalDivider(
                        thickness = TetherBorder.hairline,
                        color = TetherTheme.colors.line,
                    )
                    ListRow(
                        title =
                            stringResource(
                                R.string.github_account_scopes,
                                access.scopes.joinToString(", "),
                            ),
                    )
                }
            }
            if (access.repositoryAccess == GithubRepositoryAccess.No) {
                AccessHint(R.string.github_account_repositories_no_hint)
            }
            if (access.scopePresentation == GithubScopePresentation.FineGrained) {
                AccessHint(R.string.github_account_fine_grained_access_hint)
            }
            TetherTextButton(
                label = stringResource(R.string.github_account_sign_out),
                onClick = onSignOut,
                tone = TextButtonTone.Danger,
                modifier = Modifier.padding(top = TetherSpace.space2),
            )
        } else {
            Text(
                text = stringResource(R.string.github_account_signed_out),
                style = MaterialTheme.typography.bodyMedium,
                color = TetherTheme.colors.text2,
            )
        }
    }
}

/** A short explanation below the account details. */
@Composable
private fun AccessHint(
    @androidx.annotation.StringRes textRes: Int,
) {
    Text(
        text = stringResource(textRes),
        style = MaterialTheme.typography.bodySmall,
        color = TetherTheme.colors.text2,
        modifier = Modifier.padding(top = TetherSpace.space2),
    )
}
