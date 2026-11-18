package app.tether.shell

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
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
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.di.AppModule
import app.tether.nav.Route

/** P0 settings: only the theme choice, so light and dark can be checked by hand. */
@Composable
fun SettingsScreen(
    module: AppModule,
    onBack: () -> Unit,
) {
    val mode by module.themeMode.collectAsStateWithLifecycle()
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
            modifier = Modifier.padding(padding).padding(TetherSpace.space4).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
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
