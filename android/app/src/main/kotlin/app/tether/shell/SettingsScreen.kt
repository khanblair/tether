package app.tether.shell

import android.text.format.Formatter
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
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.R
import app.tether.design.TetherIcons
import app.tether.design.TetherTheme
import app.tether.design.ThemeMode
import app.tether.design.components.ListGroup
import app.tether.design.components.ListRow
import app.tether.design.components.TetherIcon
import app.tether.design.components.TetherTopBar
import app.tether.design.tokens.TetherSpace
import app.tether.di.AppModule
import app.tether.nav.Route
import app.tether.nav.SETTINGS_BUDGETS_ROUTE
import app.tether.nav.SETTINGS_DATA_ROUTE
import app.tether.nav.SETTINGS_EXECUTORS_ROUTE
import app.tether.nav.SETTINGS_GITHUB_ROUTE
import app.tether.nav.SETTINGS_MCP_ROUTE
import app.tether.nav.SETTINGS_MODELS_ROUTE
import app.tether.nav.SETTINGS_NOTIFICATIONS_ROUTE
import app.tether.nav.SETTINGS_PERMISSIONS_ROUTE
import app.tether.nav.SETTINGS_PRIVACY_ROUTE
import app.tether.nav.SETTINGS_PROFILE_ROUTE
import app.tether.nav.SETTINGS_QUALITY_ROUTE
import app.tether.nav.SKILLS_MEMORY_ROUTE
import app.tether.session.labelRes
import app.tether.settings.SettingsDeps
import app.tether.settings.SettingsSummary
import app.tether.settings.SettingsSummaryViewModel
import app.tether.updates.UpdatesUiState

/**
 * The main Settings list. Each group is a feature's list; each row opens its own screen. The theme
 * choice stays on this list, and Providers and About open their existing screens.
 */
@Composable
fun SettingsScreen(
    module: AppModule,
    settings: SettingsDeps,
    onBack: () -> Unit,
    onOpen: (String) -> Unit,
    onOpenProviders: () -> Unit,
    onOpenAbout: () -> Unit,
) {
    val summaryModel: SettingsSummaryViewModel =
        viewModel(
            factory =
                viewModelFactory {
                    initializer {
                        SettingsSummaryViewModel(
                            settings.store,
                            settings.notifications,
                            settings.storage,
                        )
                    }
                },
        )
    val summary by summaryModel.state.collectAsStateWithLifecycle()
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) { summaryModel.refresh() }
    val mode by module.themeMode.collectAsStateWithLifecycle()
    val updateState by module.updates.state.collectAsStateWithLifecycle()
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
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(TetherSpace.space4)
                    .fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(TetherSpace.space3),
        ) {
            MainGroups(summary, onOpen, onOpenProviders)
            Text(
                text = stringResource(R.string.settings_theme_title),
                style = MaterialTheme.typography.titleSmall,
            )
            ThemeChoices(mode, module)
            Text(
                text = stringResource(R.string.settings_theme_note),
                style = MaterialTheme.typography.bodySmall,
                color = TetherTheme.colors.text3,
            )
            ListGroup {
                ListRow(
                    title = stringResource(R.string.settings_area_list_notifications),
                    subtitle =
                        stringResource(
                            if (summary.notificationsOn) {
                                R.string.settings_area_list_notifications_on
                            } else {
                                R.string.settings_area_list_notifications_off
                            },
                        ),
                    trailing = { ChevronIcon() },
                    onClick = { onOpen(SETTINGS_NOTIFICATIONS_ROUTE) },
                )
                ListRow(
                    title = stringResource(R.string.settings_area_list_privacy),
                    subtitle =
                        stringResource(
                            if (summary.appLock) {
                                R.string.settings_area_list_privacy_on
                            } else {
                                R.string.settings_area_list_privacy_off
                            },
                        ),
                    trailing = { ChevronIcon() },
                    onClick = { onOpen(SETTINGS_PRIVACY_ROUTE) },
                )
                ListRow(
                    title = stringResource(R.string.settings_area_list_data),
                    subtitle = dataSubtitle(summary.storageBytes),
                    trailing = { ChevronIcon() },
                    onClick = { onOpen(SETTINGS_DATA_ROUTE) },
                )
                UpdatesRow(state = updateState, onClick = onOpenAbout)
            }
        }
    }
}

/**
 * The profile card and the feature groups. Every subtitle is a value the app read, or a plain
 * status.
 */
@Composable
private fun MainGroups(
    summary: SettingsSummary,
    onOpen: (String) -> Unit,
    onOpenProviders: () -> Unit,
) {
    ListGroup {
        ListRow(
            title =
                summary.profileName.ifEmpty {
                    stringResource(
                        R.string.settings_area_profile_unset_title,
                    )
                },
            subtitle = stringResource(R.string.settings_area_profile_subtitle),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_PROFILE_ROUTE) },
        )
    }
    ListGroup {
        ListRow(
            title = stringResource(R.string.providers_title),
            subtitle = stringResource(R.string.providers_settings_subtitle),
            trailing = { ChevronIcon() },
            onClick = onOpenProviders,
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_models),
            subtitle =
                summary.defaultModel?.modelId
                    ?: stringResource(R.string.settings_area_list_models_unset),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_MODELS_ROUTE) },
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_permissions),
            subtitle =
                stringResource(
                    R.string.settings_area_list_permissions_mode,
                    stringResource(summary.defaultMode.labelRes()),
                ),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_PERMISSIONS_ROUTE) },
        )
    }
    ListGroup {
        ListRow(
            title = stringResource(R.string.settings_area_list_executors),
            subtitle = stringResource(R.string.settings_area_list_executors_body),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_EXECUTORS_ROUTE) },
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_github),
            subtitle = stringResource(R.string.settings_area_not_available),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_GITHUB_ROUTE) },
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_mcp),
            subtitle = stringResource(R.string.settings_area_not_available),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_MCP_ROUTE) },
        )
    }
    ListGroup {
        ListRow(
            title = stringResource(R.string.settings_area_list_skills),
            subtitle = stringResource(R.string.settings_area_not_available),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SKILLS_MEMORY_ROUTE) },
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_quality),
            subtitle = stringResource(R.string.settings_area_not_available),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_QUALITY_ROUTE) },
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_budgets),
            subtitle = stringResource(R.string.settings_area_not_available),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_BUDGETS_ROUTE) },
        )
    }
}

@Composable
private fun dataSubtitle(bytes: Long?): String =
    bytes?.let {
        stringResource(
            R.string.settings_area_data_used,
            Formatter.formatShortFileSize(LocalContext.current, it),
        )
    } ?: stringResource(R.string.settings_area_data_unknown)

/** The theme chips. The choice applies at once and is saved. */
@Composable
private fun ThemeChoices(
    mode: ThemeMode,
    module: AppModule,
) {
    val labels =
        mapOf(
            ThemeMode.System to R.string.settings_theme_system,
            ThemeMode.Light to R.string.settings_theme_light,
            ThemeMode.Dark to R.string.settings_theme_dark,
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
}

@Composable
private fun ChevronIcon() {
    TetherIcon(
        icon = TetherIcons.ChevronRight,
        contentDescription = null,
        tint = TetherTheme.colors.text3,
    )
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
        trailing = { ChevronIcon() },
        onClick = onClick,
    )
}
