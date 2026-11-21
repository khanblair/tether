package app.tether.settings

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherTheme
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle

/**
 * The biometric app lock. Turning it on needs a passed check first, so a phone that cannot pass one
 * is never locked out. Turning it off needs no check.
 */
@Composable
fun PrivacyScreen(
    viewModel: PrivacyViewModel,
    onBack: () -> Unit,
    onTurnOnLock: (onPassed: () -> Unit) -> Unit,
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    SettingsPage(title = stringResource(R.string.settings_area_privacy_title), onBack = onBack) {
        SectionTitle(text = stringResource(R.string.settings_area_privacy_lock))
        ListGroup {
            ListRow(
                title = stringResource(R.string.settings_area_app_lock),
                subtitle = stringResource(R.string.settings_area_app_lock_body),
                trailing = {
                    Switch(
                        checked = state.appLock,
                        onCheckedChange = { on ->
                            if (on) {
                                onTurnOnLock {
                                    viewModel.setAppLock(
                                        true,
                                    )
                                }
                            } else {
                                viewModel.setAppLock(false)
                            }
                        },
                    )
                },
            )
        }
        SectionTitle(text = stringResource(R.string.settings_area_privacy_protects))
        Text(
            text = stringResource(R.string.settings_area_privacy_protects_body),
            style = MaterialTheme.typography.bodySmall,
            color = TetherTheme.colors.text2,
        )
    }
}
