package app.tether.settings

import android.content.Context
import androidx.core.content.edit
import app.tether.core.generated.ProviderView
import app.tether.session.SessionMode
import app.tether.session.ThinkingEffort

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

    /** Whether the first-run flow has been finished or skipped. False until then. */
    fun onboarded(): Boolean

    /** Saves whether the first-run flow is done. Clearing it shows the flow again. */
    fun saveOnboarded(done: Boolean)

    /** Saves the default permission mode. */
    fun saveDefaultMode(mode: SessionMode)

    /** The default thinking effort for new sessions. Medium when never set; null is the model. */
    fun defaultEffort(): ThinkingEffort?

    /** Saves the default thinking effort. Null saves the model default. */
    fun saveDefaultEffort(effort: ThinkingEffort?)

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

    override fun defaultMode(): SessionMode = savedDefaultMode() ?: SessionMode.ACCEPT_EDITS

    override fun savedDefaultMode(): SessionMode? =
        defaultModeNamed(prefs.getString(DEFAULT_MODE, null))

    override fun saveDefaultMode(mode: SessionMode) {
        prefs.edit { putString(DEFAULT_MODE, mode.name) }
    }

    override fun defaultEffort(): ThinkingEffort? = savedDefaultEffort()

    override fun savedDefaultEffort(): ThinkingEffort? =
        defaultEffortNamed(prefs.getString(DEFAULT_EFFORT, null))

    override fun saveDefaultEffort(effort: ThinkingEffort?) {
        prefs.edit { putString(DEFAULT_EFFORT, effort.storedText()) }
    }

    override fun onboarded(): Boolean = prefs.getBoolean(ONBOARDED, false)

    override fun saveOnboarded(done: Boolean) {
        prefs.edit { putBoolean(ONBOARDED, done) }
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
        const val DEFAULT_EFFORT = "default_effort"
        const val APP_LOCK = "app_lock"
        const val ONBOARDED = "onboarded"
    }
}

/**
 * The default mode named [saved]. Null when none is saved, or when the saved name is Bypass, which
 * is never a default (an older install may still hold it).
 */
internal fun defaultModeNamed(saved: String?): SessionMode? =
    SessionMode.entries.firstOrNull { it.name == saved }?.takeUnless { it == SessionMode.BYPASS }

/** The stored text for the model default. It is never a level name. */
private const val NO_EFFORT = "none"

/**
 * The default thinking effort named [saved]. A value never saved is Medium. The stored text
 * [NO_EFFORT] is the explicit model default, which reads as null. An unknown name reads as Medium.
 */
internal fun defaultEffortNamed(saved: String?): ThinkingEffort? =
    when (saved) {
        null -> ThinkingEffort.MEDIUM
        NO_EFFORT -> null
        else -> ThinkingEffort.entries.firstOrNull { it.stored() == saved } ?: ThinkingEffort.MEDIUM
    }

/** The text saved for [this] effort. Null is saved as [NO_EFFORT]. */
internal fun ThinkingEffort?.storedText(): String = this?.stored() ?: NO_EFFORT

private fun ThinkingEffort.stored(): String = name.lowercase()
