package app.tether.di

import android.content.Context
import app.tether.core.CoreClient
import app.tether.core.generated.ProviderSettings
import app.tether.core.keystoreSecretVault
import app.tether.design.ThemeMode
import app.tether.providers.CoreProviderGateway
import app.tether.providers.ProviderGateway
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

    private val themeModeState = MutableStateFlow(ThemeMode.System)

    /** The theme choice. Held in memory until settings storage ships. */
    val themeMode: StateFlow<ThemeMode> = themeModeState

    fun setThemeMode(mode: ThemeMode) {
        themeModeState.value = mode
    }
}
