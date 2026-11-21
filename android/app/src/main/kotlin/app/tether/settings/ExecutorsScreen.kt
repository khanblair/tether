package app.tether.settings

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.tether.R
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.SectionTitle

/**
 * Where commands run. The sandbox is the only executor the core has; the other two are not
 * available.
 */
@Composable
fun ExecutorsScreen(onBack: () -> Unit) {
    SettingsPage(title = stringResource(R.string.settings_area_executors_title), onBack = onBack) {
        SectionTitle(text = stringResource(R.string.settings_area_executor_default))
        ListGroup {
            ListRow(
                title = stringResource(R.string.settings_area_sandbox),
                subtitle = stringResource(R.string.settings_area_sandbox_body),
            )
        }
        SectionTitle(text = stringResource(R.string.settings_area_executor_other))
        ListGroup {
            NotAvailableRow(
                title = stringResource(R.string.settings_area_linux_mode),
                explanation = stringResource(R.string.settings_area_linux_mode_na),
            )
            NotAvailableRow(
                title = stringResource(R.string.settings_area_ci),
                explanation = stringResource(R.string.settings_area_ci_na),
            )
        }
    }
}
