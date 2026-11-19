package app.tether.updates

import android.content.Context
import androidx.core.content.edit

/** What the app remembers between launches about update checks. */
interface UpdateStore {
    /** When the last check finished, in Unix milliseconds, or null when none did. */
    fun lastCheckMs(): Long?

    /** The newest version a check found and the user has not installed, or null. */
    fun availableVersion(): String?

    /** Records a finished check. [available] is the newer version it found, or null. */
    fun recordCheck(
        atMs: Long,
        available: String?,
    )
}

/** Keeps the check time and the known newer version in a private preferences file. */
class PrefsUpdateStore(
    context: Context,
) : UpdateStore {
    private val prefs = context.getSharedPreferences(FILE, Context.MODE_PRIVATE)

    override fun lastCheckMs(): Long? =
        if (prefs.contains(LAST_CHECK)) prefs.getLong(LAST_CHECK, 0L) else null

    override fun availableVersion(): String? = prefs.getString(AVAILABLE, null)

    override fun recordCheck(
        atMs: Long,
        available: String?,
    ) = prefs.edit {
        putLong(LAST_CHECK, atMs)
        if (available == null) remove(AVAILABLE) else putString(AVAILABLE, available)
    }

    private companion object {
        const val FILE = "updates"
        const val LAST_CHECK = "last_check_ms"
        const val AVAILABLE = "available_version"
    }
}
