package app.tether.appearance

import android.content.Context
import androidx.core.content.edit
import kotlinx.coroutines.CancellationException

/** Where the appearance choices are kept between launches. */
interface AppearanceStore {
    /** The saved choices. Returns the defaults when nothing usable is saved. */
    fun load(): Appearance

    /** Saves [appearance] for the next launch. */
    fun save(appearance: Appearance)
}

private const val KEY_INTERFACE = "interface_percent"
private const val KEY_TEXT = "text_percent"
private const val KEY_ACCENT = "accent"

/**
 * Reads the choices from raw stored values. [read] returns the stored value for a key, or null.
 * A value of the wrong type, or a failed read, loads the complete default appearance.
 */
internal fun appearanceFromValues(read: (String) -> Any?): Appearance =
    try {
        val defaults = Appearance()
        Appearance(
            interfacePercent = read(KEY_INTERFACE)?.asInt() ?: defaults.interfacePercent,
            textPercent = read(KEY_TEXT)?.asInt() ?: defaults.textPercent,
            accentArgb = read(KEY_ACCENT)?.asInt(),
        ).normalized()
    } catch (failure: CancellationException) {
        throw failure
    } catch (_: ClassCastException) {
        Appearance()
    } catch (_: IllegalStateException) {
        Appearance()
    } catch (_: SecurityException) {
        Appearance()
    }

private fun Any.asInt(): Int = this as Int

/** The choices in a private preferences file. Read at once so the first frame is right. */
class PrefsAppearanceStore(
    context: Context,
) : AppearanceStore {
    private val prefs = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)

    override fun load(): Appearance = appearanceFromValues { key -> prefs.all[key] }

    override fun save(appearance: Appearance) {
        val normalized = appearance.normalized()
        prefs.edit {
            putInt(KEY_INTERFACE, normalized.interfacePercent)
            putInt(KEY_TEXT, normalized.textPercent)
            val accent = normalized.accentArgb
            if (accent == null) remove(KEY_ACCENT) else putInt(KEY_ACCENT, accent)
        }
    }

    private companion object {
        const val FILE = "appearance"
    }
}
