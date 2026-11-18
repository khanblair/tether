package app.tether.di

import app.tether.core.CoreClient
import app.tether.design.ThemeMode
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

/** Hand-wired dependencies. A DI library arrives only when this grows past a screenful. */
class AppModule {
    val core: CoreClient = CoreClient()

    private val themeModeState = MutableStateFlow(ThemeMode.System)

    /** The theme choice. Held in memory until settings storage ships. */
    val themeMode: StateFlow<ThemeMode> = themeModeState

    fun setThemeMode(mode: ThemeMode) {
        themeModeState.value = mode
    }
}
