package app.tether.settings

import java.io.File

/** Bytes used on this phone by each kind of stored data. */
data class StorageUsage(
    val database: Long,
    val workspaces: Long,
    val checkpointBlobs: Long,
    val updateCache: Long,
) {
    /** The bytes that the app keeps for projects and sessions, excluding the update cache. */
    val appData: Long get() = database + workspaces + checkpointBlobs
}

/**
 * Reads the storage sizes and clears the update cache. Calls touch the disk, so run them off the
 * main thread.
 */
interface StorageSource {
    /** The current sizes. */
    fun usage(): StorageUsage

    /**
     * Deletes what is in the update cache. The folder itself stays, since the updater uses its
     * path. False when a file stayed.
     */
    fun clearUpdateCache(): Boolean
}

/**
 * The real source over the app's files. The database counts with its write-ahead log and shared
 * memory.
 */
class FileStorageSource(
    private val databaseFile: File,
    private val workspaces: File,
    private val checkpointBlobs: File,
    private val updateCache: File,
) : StorageSource {
    override fun usage(): StorageUsage =
        StorageUsage(
            database = DATABASE_SUFFIXES.sumOf { directorySize(File(databaseFile.path + it)) },
            workspaces = directorySize(workspaces),
            checkpointBlobs = directorySize(checkpointBlobs),
            updateCache = directorySize(updateCache),
        )

    override fun clearUpdateCache(): Boolean =
        updateCache
            .listFiles()
            .orEmpty()
            .map { it.deleteRecursively() }
            .all { it }

    private companion object {
        val DATABASE_SUFFIXES = listOf("", "-wal", "-shm")
    }
}

/**
 * The size of [file], or the sum of every file under it when it is a folder. A missing path is 0.
 */
fun directorySize(file: File): Long =
    when {
        file.isFile -> file.length()
        file.isDirectory -> file.walkTopDown().filter(File::isFile).sumOf(File::length)
        else -> 0L
    }
