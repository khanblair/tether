package app.tether.crash

import java.io.File
import java.io.IOException

/** The saved report of the last crash, if there is one. */
interface CrashStore {
    /** The saved report, or null when there is none or it cannot be read. */
    fun read(): String?

    /** Removes the saved report. */
    fun clear()
}

/** Keeps the report in [dir] as `last.txt`. */
class FileCrashStore(
    private val dir: File,
) : CrashStore {
    private val file: File
        get() = dir.resolve(FILE_NAME)

    override fun read(): String? =
        try {
            file.takeIf { it.isFile }?.readText()
        } catch (_: IOException) {
            null
        }

    override fun clear() {
        file.delete()
    }

    /** Saves [text] as the last report. It creates the folder when it is missing. */
    fun write(text: String) {
        dir.mkdirs()
        file.writeText(text)
    }

    companion object {
        const val FILE_NAME = "last.txt"
    }
}
