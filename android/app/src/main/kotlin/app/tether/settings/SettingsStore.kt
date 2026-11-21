package app.tether.settings

import android.content.Context
import androidx.core.content.edit
import app.tether.core.generated.ProviderView
import app.tether.session.SessionMode

/** The settings the app keeps on this phone. Nothing here is sent anywhere. */
interface SettingsStore : AppDefaultSource {
    /** The display name, or an empty string when none is saved. */
    fun profileName(): String

    /** Saves the display name. The caller trims it. */
    fun saveProfileName(name: String)

    /** Saves the app default model. Null clears it. */
    fun saveDefaultModel(model: DefaultModel?)

    /** The default permission mode for new sessions. Accept edits when none is saved. */
    fun defaultMode(): SessionMode

    /** Saves the default permission mode. */
    fun saveDefaultMode(mode: SessionMode)

    /** Whether the biometric app lock is on. */
    fun appLockEnabled(): Boolean

    /** Turns the biometric app lock on or off. */
    fun saveAppLockEnabled(enabled: Boolean)
}

/** The settings in a private preferences file. An unknown saved value reads as its default. */
class PrefsSettingsStore(
    context: Context,
) : SettingsStore {
    private val prefs = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)

    override fun profileName(): String = prefs.getString(PROFILE_NAME, null).orEmpty()

    override fun saveProfileName(name: String) {
        prefs.edit { putString(PROFILE_NAME, name) }
    }

    override fun defaultModel(): DefaultModel? {
        val provider = prefs.getString(DEFAULT_PROVIDER, null)?.let(::providerNamed)
        val modelId = prefs.getString(DEFAULT_MODEL, null)?.takeIf(String::isNotBlank)
        return if (provider != null && modelId != null) DefaultModel(provider, modelId) else null
    }

    override fun saveDefaultModel(model: DefaultModel?) {
        prefs.edit {
            if (model == null) {
                remove(DEFAULT_PROVIDER)
                remove(DEFAULT_MODEL)
            } else {
                putString(DEFAULT_PROVIDER, model.provider.name)
                putString(DEFAULT_MODEL, model.modelId)
            }
        }
    }

    override fun defaultMode(): SessionMode {
        val saved = prefs.getString(DEFAULT_MODE, null)
        return SessionMode.entries.firstOrNull { it.name == saved } ?: SessionMode.ACCEPT_EDITS
    }

    override fun saveDefaultMode(mode: SessionMode) {
        prefs.edit { putString(DEFAULT_MODE, mode.name) }
    }

    override fun appLockEnabled(): Boolean = prefs.getBoolean(APP_LOCK, false)

    override fun saveAppLockEnabled(enabled: Boolean) {
        prefs.edit { putBoolean(APP_LOCK, enabled) }
    }

    private fun providerNamed(name: String): ProviderView? =
        ProviderView.entries.firstOrNull { it.name == name }

    private companion object {
        const val FILE = "settings"
        const val PROFILE_NAME = "profile_name"
        const val DEFAULT_PROVIDER = "default_provider"
        const val DEFAULT_MODEL = "default_model"
        const val DEFAULT_MODE = "default_mode"
        const val APP_LOCK = "app_lock"
    }
}
