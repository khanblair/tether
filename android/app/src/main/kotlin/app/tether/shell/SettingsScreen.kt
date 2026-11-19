package app.tether.shell

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.di.AppModule
import app.tether.nav.Route
import app.tether.updates.UpdatesUiState

/** The entry to model providers, the About screen, and the theme choice. */
@Composable
fun SettingsScreen(
    module: AppModule,
    onBack: () -> Unit,
    onOpenProviders: () -> Unit,
    onOpenAbout: () -> Unit,
) {
    val mode by module.themeMode.collectAsStateWithLifecycle()
    val updateState by module.updates.state.collectAsStateWithLifecycle()
    val labels =
        mapOf(
            ThemeMode.System to R.string.settings_theme_system,
            ThemeMode.Light to R.string.settings_theme_light,
            ThemeMode.Dark to R.string.settings_theme_dark,
        )

    Scaffold(
        containerColor = TetherTheme.colors.bg,
        topBar = {
            TetherTopBar(
                title = stringResource(Route.Settings.title),
                navigationIcon = { BackButton(onBack) },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .padding(
                        padding,
                    ).verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4)
                    .fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            ListRow(
                title = stringResource(R.string.providers_title),
                subtitle = stringResource(R.string.providers_settings_subtitle),
                trailing = {
                    TetherIcon(
                        icon = TetherIcons.ChevronRight,
                        contentDescription = null,
                        tint = TetherTheme.colors.text3,
                    )
                },
                onClick = onOpenProviders,
            )
            UpdatesRow(state = updateState, onClick = onOpenAbout)
            Text(
                text = stringResource(R.string.settings_theme_title),
                style = MaterialTheme.typography.titleSmall,
            )
            Column(verticalArrangement = Arrangement.spacedBy(TetherSpace.space2)) {
                ThemeMode.entries.forEach { option ->
                    FilterChip(
                        selected = option == mode,
                        onClick = { module.setThemeMode(option) },
                        label = { Text(text = stringResource(labels.getValue(option))) },
                    )
                }
            }
            Text(
                text = stringResource(R.string.settings_theme_note),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
        }
    }
}

/** The About and updates entry. Its subtitle names the new version when one is known. */
@Composable
private fun UpdatesRow(
    state: UpdatesUiState,
    onClick: () -> Unit,
) {
    val version = state.newerVersion
    val subtitle =
        if (state.updateAvailable && version != null) {
            stringResource(R.string.settings_about_update_available, version)
        } else {
            stringResource(R.string.settings_about_version, state.currentVersion)
        }
    ListRow(
        title = stringResource(R.string.settings_about_title),
        subtitle = subtitle,
        trailing = {
            TetherIcon(
                icon = TetherIcons.ChevronRight,
                contentDescription = null,
                tint = TetherTheme.colors.text3,
            )
        },
        onClick = onClick,
    )
}
