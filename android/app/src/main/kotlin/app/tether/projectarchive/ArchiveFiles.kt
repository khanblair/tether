package app.tether.projectarchive

import android.content.Context
import android.database.Cursor
import android.net.Uri
import android.provider.OpenableColumns
import java.io.File
import java.io.IOException
import java.util.UUID

/** A content URI as text, so view models never touch Android types. */
@JvmInline
value class ArchiveUri(
    val raw: String,
)

/** Converts a picker result to an [ArchiveUri]. */
fun Uri.toArchiveUri(): ArchiveUri = ArchiveUri(toString())

/** A file in the app's cache folder. The core reads and writes it by [path]. */
data class TempFile(
    val path: String,
)

/**
 * Copies between content URIs and the cache folder, because the core reads and writes plain file
 * paths. Each function blocks, so callers run it off the main thread. The fake in unit tests
 * records calls instead of touching storage.
 */
interface ArchiveFiles {
    /** Copies the content at [uri] into a new import file. Throws when it cannot. */
    fun copyIn(uri: ArchiveUri): TempFile

    /** A new, empty file under the export folder, for the core to write into. */
    fun reserveExport(): TempFile

    /** Copies [temp] to the content at [uri], replacing what was there. Throws when it cannot. */
    fun copyOut(
        temp: TempFile,
        uri: ArchiveUri,
    )

    /** Deletes [temp]. Missing files are not an error. */
    fun delete(temp: TempFile)

    /** The display name the provider reports for [uri], or null when it has none. */
    fun displayName(uri: ArchiveUri): String?

    /** The size the provider reports for [uri] in bytes, or null when it is unknown. */
    fun sizeBytes(uri: ArchiveUri): Long?
}

/** The real [ArchiveFiles]: copies through the [android.content.ContentResolver]. */
class ContentResolverArchiveFiles(
    context: Context,
) : ArchiveFiles {
    private val resolver = context.contentResolver
    private val cacheRoot = context.cacheDir

    override fun copyIn(uri: ArchiveUri): TempFile {
        val target = newCacheFile(IMPORT_FOLDER)
        try {
            val input =
                resolver.openInputStream(Uri.parse(uri.raw))
                    ?: throw IOException("The provider returned no stream.")
            input.use { source -> target.outputStream().use { source.copyTo(it) } }
        } catch (error: IOException) {
            target.delete()
            throw error
        }
        return TempFile(target.absolutePath)
    }

    override fun reserveExport(): TempFile = TempFile(newCacheFile(EXPORT_FOLDER).absolutePath)

    override fun copyOut(
        temp: TempFile,
        uri: ArchiveUri,
    ) {
        val output =
            resolver.openOutputStream(Uri.parse(uri.raw), "wt")
                ?: throw IOException("The provider returned no stream.")
        output.use { sink -> File(temp.path).inputStream().use { it.copyTo(sink) } }
    }

    override fun delete(temp: TempFile) {
        File(temp.path).delete()
    }

    override fun displayName(uri: ArchiveUri): String? =
        queryColumn(uri, OpenableColumns.DISPLAY_NAME) { cursor, column ->
            cursor.getString(column)
        }

    override fun sizeBytes(uri: ArchiveUri): Long? =
        queryColumn(uri, OpenableColumns.SIZE) { cursor, column ->
            cursor.getLong(column).takeIf { it >= 0 }
        }

    /** Reads one column of the first row the provider returns, or null when there is none. */
    private fun <T> queryColumn(
        uri: ArchiveUri,
        column: String,
        read: (Cursor, Int) -> T?,
    ): T? {
        val query = resolver.query(Uri.parse(uri.raw), arrayOf(column), null, null, null)
        val cursor = query ?: return null
        return cursor.use {
            val index = it.getColumnIndex(column)
            if (index >= 0 && it.moveToFirst() && !it.isNull(index)) read(it, index) else null
        }
    }

    private fun newCacheFile(folder: String): File {
        val directory = File(cacheRoot, folder).apply { mkdirs() }
        return File(directory, "${UUID.randomUUID()}.zip")
    }

    private companion object {
        const val IMPORT_FOLDER = "imports"
        const val EXPORT_FOLDER = "exports"
    }
}
