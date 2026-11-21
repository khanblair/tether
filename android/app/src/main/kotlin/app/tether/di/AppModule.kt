package app.tether.di

import android.content.Context
import app.tether.core.CoreClient
import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProviderSettings
import app.tether.core.generated.Updater
import app.tether.core.keystoreSecretVault
import app.tether.design.ThemeMode
import app.tether.home.CoreProjectGateway
import app.tether.home.HomeSources
import app.tether.home.ProjectGateway
import app.tether.home.createHomeSources
import app.tether.messages.AppMessages
import app.tether.providers.CoreProviderGateway
import app.tether.providers.ProviderGateway
import app.tether.session.CoreSessionGateway
import app.tether.session.SessionGateway
import app.tether.session.SessionSettingsGateway
import app.tether.session.SessionStarter
import app.tether.session.createSessionSettingsGateway
import app.tether.settings.PrefsSettingsStore
import app.tether.settings.createProjectDefaultsGateway
import app.tether.share.ShareInbox
import app.tether.theme.PrefsThemeStore
import app.tether.theme.ThemeController
import app.tether.updates.CoreUpdateGateway
import app.tether.updates.PrefsUpdateStore
import app.tether.updates.UpdatesController
import app.tether.updates.createUpdateInstaller
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.StateFlow

/** Hand-wired dependencies. A DI library arrives only when this grows past a screenful. */
class AppModule(
    context: Context,
) {
    val core: CoreClient = CoreClient()
    val databasePath: String = context.getDatabasePath("tether.db").absolutePath
    val workspacesRoot: String =
        context.filesDir
            .resolve("workspaces")
            .apply { mkdirs() }
            .absolutePath
    val vault = keystoreSecretVault(context.filesDir.resolve("secrets"))
    val shareInbox = ShareInbox()

    /** The provider database opens on the first call, which runs off the main thread. */
    val providerGateway: ProviderGateway =
        CoreProviderGateway(
            lazy { ProviderSettings(context.getDatabasePath("tether.db").absolutePath, vault) },
        )

    /** The project list and creation. The runtime opens on the first call, off the main thread. */
    val projectGateway: ProjectGateway =
        CoreProjectGateway(
            lazy { ProjectRuntime(databasePath, workspacesRoot) },
        )

    /** Session lists and creation. The runtime opens on the first call, off the main thread. */
    val sessionGateway: SessionGateway by lazy { CoreSessionGateway(databasePath, vault) }

    /** Model, mode, rename, usage, rewind, and message search for the session screens. */
    val sessionSettings: SessionSettingsGateway by lazy {
        createSessionSettingsGateway(databasePath, vault)
    }

    /** Starts a session in a project, choosing the first provider that has a key. */
    val sessionStarter: SessionStarter by lazy {
        SessionStarter(
            providerGateway,
            sessionGateway,
            projectDefaults = createProjectDefaultsGateway(databasePath, workspacesRoot),
            appDefaults = PrefsSettingsStore(context),
        )
    }

    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** The data behind the home screens: projects and sessions. Debug and release builds differ. */
    val home: HomeSources = createHomeSources(projectGateway, sessionGateway)

    /** The update flow, shared by the Needs attention list and the About screen. */
    val updates: UpdatesController by lazy {
        UpdatesController(
            gateway =
                CoreUpdateGateway(
                    lazy { Updater(context.cacheDir.resolve("updates").absolutePath) },
                ),
            installer = createUpdateInstaller(context),
            store = PrefsUpdateStore(context),
            currentVersion =
                context.packageManager
                    .getPackageInfo(
                        context.packageName,
                        0,
                    ).versionName
                    .orEmpty(),
            scope = appScope,
        )
    }

    /** The theme choice. It is saved in preferences and applies at once. */
    val theme = ThemeController(PrefsThemeStore(context))

    /** The theme choice, read by the activity. */
    val themeMode: StateFlow<ThemeMode> = theme.mode

    fun setThemeMode(mode: ThemeMode) {
        theme.set(mode)
    }

    /** Messages for the snackbar host, such as the toast after a test event. */
    val messages = AppMessages()
}
