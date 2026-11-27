package app.tether.settings

import app.tether.workspace.WorkspaceDataPaths
import java.io.File

/** Bytes used on this phone by each kind of stored data. */
data class StorageUsage(
    val database: Long,
    val workspaces: Long,
    val checkpointBlobs: Long,
    val updateCache: Long,
    val secrets: Long = 0L,
    val memory: Long = 0L,
    val skills: Long = 0L,
    val vault: Long = 0L,
    val crashReport: Long = 0L,
) {
    /** The bytes that the app keeps for projects and sessions, excluding the update cache. */
    val appData: Long
        get() =
            database +
                workspaces +
                checkpointBlobs +
                secrets +
                memory +
                skills +
                vault +
                crashReport
}

/** The kinds of local data shown on the Data screen. */
enum class DataLocationKind {
    Projects,
    Database,
    Secrets,
    Checkpoints,
    Memory,
    Skills,
    Vault,
    CrashReport,
}

/** One read-only local data location with its path and size. */
data class DataLocation(
    val kind: DataLocationKind,
    val path: String,
    val size: Long,
)

/** Builds the fixed-order data list used by Settings and tested without Android UI classes. */
fun dataLocations(
    paths: WorkspaceDataPaths,
    usage: StorageUsage,
): List<DataLocation> =
    listOf(
        DataLocation(DataLocationKind.Projects, paths.projects, usage.workspaces),
        DataLocation(DataLocationKind.Database, paths.database, usage.database),
        DataLocation(DataLocationKind.Secrets, paths.secrets, usage.secrets),
        DataLocation(DataLocationKind.Checkpoints, paths.checkpoints, usage.checkpointBlobs),
        DataLocation(DataLocationKind.Memory, paths.memory, usage.memory),
        DataLocation(DataLocationKind.Skills, paths.skills, usage.skills),
        DataLocation(DataLocationKind.Vault, paths.vault, usage.vault),
        DataLocation(DataLocationKind.CrashReport, paths.crashReport, usage.crashReport),
    )

/**
 * Reads the storage sizes and clears the update cache. Calls touch the disk, so run them off the
 * main thread.
 */
interface StorageSource {
    /** The current sizes. */
    fun usage(): StorageUsage

    /** The paths shown with the data rows. */
    fun paths(): WorkspaceDataPaths = WorkspaceDataPaths()

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
    private val workspaceRoot: (() -> File)? = null,
    private val secrets: File? = null,
    private val memory: File? = null,
    private val skills: File? = null,
    private val vault: File? = null,
    private val crashReport: File? = null,
) : StorageSource {
    override fun usage(): StorageUsage =
        StorageUsage(
            database = DATABASE_SUFFIXES.sumOf { directorySize(File(databaseFile.path + it)) },
            workspaces = directorySize(workspaceRoot?.invoke() ?: workspaces),
            checkpointBlobs = directorySize(checkpointBlobs),
            updateCache = directorySize(updateCache),
            secrets = optionalDirectorySize(secrets),
            memory = optionalDirectorySize(memory),
            skills = optionalDirectorySize(skills),
            vault = optionalDirectorySize(vault),
            crashReport = optionalDirectorySize(crashReport),
        )

    override fun paths(): WorkspaceDataPaths =
        WorkspaceDataPaths(
            projects = (workspaceRoot?.invoke() ?: workspaces).absolutePath,
            database = databaseFile.absolutePath,
            secrets = secrets?.absolutePath.orEmpty(),
            checkpoints = checkpointBlobs.absolutePath,
            memory = memory?.absolutePath.orEmpty(),
            skills = skills?.absolutePath.orEmpty(),
            vault = vault?.absolutePath.orEmpty(),
            crashReport = crashReport?.absolutePath.orEmpty(),
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

private fun optionalDirectorySize(file: File?): Long = file?.let(::directorySize) ?: 0L
