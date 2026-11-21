package app.tether.nav

import android.content.Context
import android.content.Intent
import android.provider.Settings
import androidx.activity.compose.LocalActivity
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import androidx.navigation.NavGraphBuilder
import androidx.navigation.compose.composable
import app.tether.MainActivity
import app.tether.R
import app.tether.TetherApp
import app.tether.di.AppModule
import app.tether.settings.DataScreen
import app.tether.settings.DataViewModel
import app.tether.settings.ExecutorsScreen
import app.tether.settings.ModelDefaultsScreen
import app.tether.settings.ModelDefaultsViewModel
import app.tether.settings.NotificationsScreen
import app.tether.settings.NotificationsViewModel
import app.tether.settings.PermissionsScreen
import app.tether.settings.PermissionsViewModel
import app.tether.settings.PrivacyScreen
import app.tether.settings.PrivacyViewModel
import app.tether.settings.ProfileScreen
import app.tether.settings.ProfileViewModel
import app.tether.settings.SettingsDeps
import app.tether.shell.PlaceholderScreen

/** The settings area's routes. Each row of the main Settings list opens one of these. */
const val SETTINGS_PROFILE_ROUTE = "settings/profile"
const val SETTINGS_MODELS_ROUTE = "settings/models"
const val SETTINGS_PERMISSIONS_ROUTE = "settings/permissions"
const val SETTINGS_EXECUTORS_ROUTE = "settings/executors"
const val SETTINGS_GITHUB_ROUTE = "settings/github"
const val SETTINGS_MCP_ROUTE = "settings/mcp"
const val SETTINGS_QUALITY_ROUTE = "settings/quality"
const val SETTINGS_BUDGETS_ROUTE = "settings/budgets"
const val SETTINGS_NOTIFICATIONS_ROUTE = "settings/notifications"
const val SETTINGS_PRIVACY_ROUTE = "settings/privacy"
const val SETTINGS_DATA_ROUTE = "settings/data"

/** Skills and memory. Area A3 changes this constant when its screens exist. */
const val SKILLS_MEMORY_ROUTE = "settings/skills-memory"

/**
 * Registers every screen of the settings area except the main list, which [settingsRoutes] owns.
 */
internal fun NavGraphBuilder.settingsAreaRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    profileAndModelRoutes(module, nav)
    permissionRoutes(module, nav)
    notAvailableRoutes(nav)
    systemRoutes(module, nav)
}

/** The profile and the models screens. */
private fun NavGraphBuilder.profileAndModelRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_PROFILE_ROUTE) {
        val deps = settingsDepsOf(LocalContext.current)
        ProfileScreen(
            viewModel =
                viewModel(
                    factory = viewModelFactory { initializer { ProfileViewModel(deps.store) } },
                ),
            onBack = nav::back,
        )
    }
    composable(SETTINGS_MODELS_ROUTE) {
        val deps = settingsDepsOf(LocalContext.current)
        ModelDefaultsScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                ModelDefaultsViewModel(
                                    deps.store,
                                    module.providerGateway,
                                    deps.models,
                                    module.projectGateway,
                                    deps.projectDefaults,
                                )
                            }
                        },
                ),
            onBack = nav::back,
            onOpenProviders = { nav.go(Route.Providers) },
        )
    }
}

/** The permissions screen. Bypass needs the same biometric check as the session's mode sheet. */
private fun NavGraphBuilder.permissionRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_PERMISSIONS_ROUTE) {
        val deps = settingsDepsOf(LocalContext.current)
        val activity = LocalActivity.current as? MainActivity
        val bypassTitle = stringResource(R.string.session_bypass_title)
        PermissionsScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                PermissionsViewModel(
                                    store = deps.store,
                                    projects = module.projectGateway,
                                    defaults = deps.projectDefaults,
                                    authenticate = { onPassed ->
                                        activity?.authenticateForKeyReveal(
                                            title = bypassTitle,
                                            onSuccess = onPassed,
                                            onError = {},
                                        )
                                    },
                                )
                            }
                        },
                ),
            onBack = nav::back,
        )
    }
}

/** The screens the app cannot show yet, each with one sentence on what it needs. */
private fun NavGraphBuilder.notAvailableRoutes(nav: TetherNav) {
    composable(SETTINGS_EXECUTORS_ROUTE) { ExecutorsScreen(onBack = nav::back) }
    composable(SETTINGS_GITHUB_ROUTE) {
        NotAvailable(
            nav,
            R.string.settings_area_github_title,
            R.string.settings_area_github_na_body,
        )
    }
    composable(SETTINGS_MCP_ROUTE) {
        NotAvailable(nav, R.string.settings_area_mcp_title, R.string.settings_area_mcp_na_body)
    }
    composable(SETTINGS_QUALITY_ROUTE) {
        NotAvailable(
            nav,
            R.string.settings_area_quality_title,
            R.string.settings_area_quality_na_body,
        )
    }
    composable(SETTINGS_BUDGETS_ROUTE) {
        NotAvailable(
            nav,
            R.string.settings_area_budgets_title,
            R.string.settings_area_budgets_na_body,
        )
    }
}

/** The notifications, privacy, and data screens. */
private fun NavGraphBuilder.systemRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_NOTIFICATIONS_ROUTE) {
        val deps = settingsDepsOf(LocalContext.current)
        val context = LocalContext.current
        NotificationsScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer { NotificationsViewModel(deps.notifications) }
                        },
                ),
            onBack = nav::back,
            onOpenChannel = { channelId ->
                context.startActivity(channelSettingsIntent(context, channelId))
            },
            onOpenAppSettings = { context.startActivity(appNotificationSettingsIntent(context)) },
        )
    }
    composable(SETTINGS_PRIVACY_ROUTE) {
        val deps = settingsDepsOf(LocalContext.current)
        val activity = LocalActivity.current as? MainActivity
        val lockTitle = stringResource(R.string.settings_area_lock_prompt)
        PrivacyScreen(
            viewModel =
                viewModel(
                    factory = viewModelFactory { initializer { PrivacyViewModel(deps.store) } },
                ),
            onBack = nav::back,
            onTurnOnLock = { onPassed ->
                activity?.authenticateForKeyReveal(
                    title = lockTitle,
                    onSuccess = onPassed,
                    onError = {},
                )
            },
        )
    }
    dataRoutes(module, nav)
}

/** The data screen. Its clear action confirms through the snackbar host. */
private fun NavGraphBuilder.dataRoutes(
    module: AppModule,
    nav: TetherNav,
) {
    composable(SETTINGS_DATA_ROUTE) {
        val deps = settingsDepsOf(LocalContext.current)
        val cleared = stringResource(R.string.settings_area_cache_cleared)
        DataScreen(
            viewModel =
                viewModel(
                    factory =
                        viewModelFactory {
                            initializer {
                                DataViewModel(
                                    deps.storage,
                                    onCacheCleared = { module.messages.show(cleared) },
                                )
                            }
                        },
                ),
            onBack = nav::back,
        )
    }
}

/** The settings of the running app, created once by [TetherApp]. */
internal fun settingsDepsOf(context: Context): SettingsDeps =
    (context.applicationContext as TetherApp).settings

@Composable
private fun NotAvailable(
    nav: TetherNav,
    titleRes: Int,
    explanationRes: Int,
) {
    PlaceholderScreen(
        title = stringResource(titleRes),
        explanation = stringResource(explanationRes),
        onBack = nav::back,
    )
}

/** The system screen for one notification channel of this app. */
private fun channelSettingsIntent(
    context: Context,
    channelId: String,
): Intent =
    Intent(Settings.ACTION_CHANNEL_NOTIFICATION_SETTINGS)
        .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
        .putExtra(Settings.EXTRA_CHANNEL_ID, channelId)

/** The system notification screen for this app. */
private fun appNotificationSettingsIntent(context: Context): Intent =
    Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
        .putExtra(Settings.EXTRA_APP_PACKAGE, context.packageName)
