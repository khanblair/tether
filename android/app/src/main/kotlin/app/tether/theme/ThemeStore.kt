package app.tether.theme

import android.content.Context
import androidx.core.content.edit
import app.tether.design.ThemeMode
import app.tether.design.next
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Where the theme choice is kept between launches. */
interface ThemeStore {
    /** The saved choice. Returns [ThemeMode.System] when none is saved. */
    fun load(): ThemeMode

    /** Saves [mode] for the next launch. */
    fun save(mode: ThemeMode)
}

/** The theme choice in a private preferences file. An unknown value loads as System. */
class PrefsThemeStore(
    context: Context,
) : ThemeStore {
    private val prefs = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)

    override fun load(): ThemeMode {
        val saved = prefs.getString(KEY, null)
        return ThemeMode.entries.firstOrNull { it.name == saved } ?: ThemeMode.System
    }

    override fun save(mode: ThemeMode) {
        prefs.edit { putString(KEY, mode.name) }
    }

    private companion object {
        const val FILE = "theme"
        const val KEY = "mode"
    }
}

/** Holds the theme choice for the whole app. Every change is saved at once. */
class ThemeController(
    private val store: ThemeStore,
) {
    private val state = MutableStateFlow(store.load())

    /** The current choice. It starts from the saved value. */
    val mode: StateFlow<ThemeMode> = state.asStateFlow()

    /** Sets the choice and saves it. */
    fun set(mode: ThemeMode) {
        state.value = mode
        store.save(mode)
    }

    /** Moves to the next choice: System, Light, Dark, then System. */
    fun cycle() {
        set(mode.value.next())
    }
}
