package app.tether.di

import android.content.Context
import app.tether.core.CoreClient
import app.tether.core.generated.ProviderSettings
import app.tether.core.generated.Updater
import app.tether.core.keystoreSecretVault
import app.tether.design.ThemeMode
import app.tether.home.HomeSources
import app.tether.home.createHomeSources
import app.tether.providers.CoreProviderGateway
import app.tether.providers.ProviderGateway
import app.tether.updates.CoreUpdateGateway
import app.tether.updates.PrefsUpdateStore
import app.tether.updates.UpdatesController
import app.tether.updates.createUpdateInstaller
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/** Hand-wired dependencies. A DI library arrives only when this grows past a screenful. */
class AppModule(
    context: Context,
) {
    val core: CoreClient = CoreClient()
    val vault = keystoreSecretVault(context.filesDir.resolve("secrets"))

    /** The provider database opens on the first call, which runs off the main thread. */
    val providerGateway: ProviderGateway =
        CoreProviderGateway(
            lazy { ProviderSettings(context.getDatabasePath("tether.db").absolutePath, vault) },
        )

    private val appScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** The data behind the home screens: projects and sessions. Debug and release builds differ. */
    val home: HomeSources = createHomeSources()

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

    private val themeModeState = MutableStateFlow(ThemeMode.System)

    /** The theme choice. Held in memory until settings storage ships. */
    val themeMode: StateFlow<ThemeMode> = themeModeState

    fun setThemeMode(mode: ThemeMode) {
        themeModeState.value = mode
    }
}
