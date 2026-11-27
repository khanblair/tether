package app.tether.shell

import android.text.format.Formatter
import androidx.annotation.PluralsRes
import androidx.annotation.StringRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Scaffold
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LifecycleEventEffect
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import app.tether.R
import app.tether.appearance.Appearance
import app.tether.appearance.AppearanceSummaryLabels
import app.tether.appearance.TEXT_FOLLOW_SYSTEM
import app.tether.appearance.appearanceRowSummary
import app.tether.design.Palette
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
import app.tether.nav.SETTINGS_APPEARANCE_ROUTE
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
import app.tether.settings.RowText
import app.tether.settings.SettingsDeps
import app.tether.settings.SettingsRowsState
import app.tether.settings.SettingsRowsViewModel
import app.tether.settings.SettingsSummary
import app.tether.settings.SettingsSummaryViewModel
import app.tether.settings.budgets.dollarsText
import app.tether.updates.UpdatesUiState

/**
 * The main Settings list. Each group is a feature's list; each row opens its own screen.
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
    val rowsModel: SettingsRowsViewModel =
        viewModel(
            factory =
                viewModelFactory {
                    initializer {
                        SettingsRowsViewModel(
                            module.githubGateway,
                            module.mcpGateway,
                            module.memoryArea.skills,
                            module.budgetsGateway,
                        )
                    }
                },
        )
    val rows by rowsModel.state.collectAsStateWithLifecycle()
    LifecycleEventEffect(Lifecycle.Event.ON_RESUME) {
        summaryModel.refresh()
        rowsModel.refresh()
    }
    val mode by module.themeMode.collectAsStateWithLifecycle()
    val appearance by module.appearance.appearance.collectAsStateWithLifecycle()
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
            MainGroups(summary, rows, onOpen, onOpenProviders)
            ListGroup {
                ListRow(
                    title = stringResource(R.string.appearance_title),
                    subtitle = appearanceSummary(mode, appearance),
                    leading = {
                        TetherIcon(
                            icon = TetherIcons.Palette,
                            contentDescription = null,
                            tint = TetherTheme.colors.text2,
                        )
                    },
                    trailing = { ChevronIcon() },
                    onClick = { onOpen(SETTINGS_APPEARANCE_ROUTE) },
                )
            }
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
    rows: SettingsRowsState,
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
            subtitle = rowSubtitle(rows.github),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_GITHUB_ROUTE) },
        )
        ListRow(
            title = stringResource(R.string.settings_area_list_mcp),
            subtitle = rowSubtitle(rows.mcp),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_MCP_ROUTE) },
        )
    }
    ListGroup {
        ListRow(
            title = stringResource(R.string.settings_area_list_skills),
            subtitle = rowSubtitle(rows.skills),
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
            subtitle = rowSubtitle(rows.budgets),
            trailing = { ChevronIcon() },
            onClick = { onOpen(SETTINGS_BUDGETS_ROUTE) },
        )
    }
}

/** The words for a row's live state. Null while the row loads or its read failed. */
@Composable
private fun rowSubtitle(text: RowText): String? =
    when (text) {
        RowText.None -> null
        is RowText.SignedIn -> signedInText(text.login)
        RowText.NotSignedIn -> stringResource(R.string.settings_rows_github_signed_out)
        is RowText.Servers ->
            countText(
                text.count,
                R.string.settings_rows_mcp_none,
                R.plurals.settings_rows_mcp_servers,
            )
        is RowText.Skills ->
            countText(
                text.count,
                R.string.settings_rows_skills_none,
                R.plurals.settings_rows_skills,
            )
        RowText.NoLimits -> stringResource(R.string.settings_rows_budgets_none)
        is RowText.DailyLimit ->
            stringResource(R.string.settings_rows_budgets_daily, dollarsText(text.micros))
        is RowText.SessionLimit ->
            stringResource(R.string.settings_rows_budgets_session, dollarsText(text.micros))
    }

@Composable
private fun signedInText(login: String?): String =
    login?.let { stringResource(R.string.settings_rows_github_signed_in_as, it) }
        ?: stringResource(R.string.settings_rows_github_signed_in)

/** The "none" words for zero, or the plural count for any other number. */
@Composable
private fun countText(
    count: Int,
    @StringRes noneRes: Int,
    @PluralsRes pluralRes: Int,
): String =
    if (count == 0) {
        stringResource(noneRes)
    } else {
        pluralStringResource(pluralRes, count, count)
    }

@Composable
private fun dataSubtitle(bytes: Long?): String =
    bytes?.let {
        stringResource(
            R.string.settings_area_data_used,
            Formatter.formatShortFileSize(LocalContext.current, it),
        )
    } ?: stringResource(R.string.settings_area_data_unknown)

@Composable
private fun appearanceSummary(
    mode: ThemeMode,
    appearance: Appearance,
): String {
    val themeName =
        stringResource(
            when (mode) {
                ThemeMode.System -> R.string.settings_theme_system
                ThemeMode.Light -> R.string.settings_theme_light
                ThemeMode.Dark -> R.string.settings_theme_dark
            },
        )
    val themeSummary = stringResource(R.string.appearance_theme_summary, themeName)
    val textLabel =
        if (appearance.textPercent == TEXT_FOLLOW_SYSTEM) {
            stringResource(R.string.appearance_text_follows_system)
        } else {
            stringResource(R.string.appearance_text_summary, appearance.textPercent)
        }
    val interfaceLabel =
        stringResource(R.string.appearance_interface_summary, appearance.interfacePercent)
    return appearanceRowSummary(
        mode = mode,
        appearance = appearance,
        labels =
            AppearanceSummaryLabels(
                theme = { themeSummary },
                followsSystem = textLabel,
                textPercent = { textLabel },
                interfacePercent = { interfaceLabel },
            ),
    )
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
        when {
            state.developmentBuild -> stringResource(R.string.dev_build_settings_subtitle)
            state.updateAvailable && version != null ->
                stringResource(R.string.settings_about_update_available, version)
            else -> stringResource(R.string.settings_about_version, state.currentVersion)
        }
    ListRow(
        title = stringResource(R.string.settings_about_title),
        subtitle = subtitle,
        trailing = { ChevronIcon() },
        onClick = onClick,
    )
}
