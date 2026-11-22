package app.tether.files

import app.tether.core.generated.FileEntryView
import app.tether.core.generated.FilesRuntime
import app.tether.core.generated.FilesRuntimeException
import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProjectRuntimeException

/** One child of a folder, as the files screens show it. [sizeBytes] is zero for folders. */
data class FileEntry(
    val name: String,
    val path: String,
    val isDir: Boolean,
    val sizeBytes: Long,
    val ignored: Boolean,
)

/** The children of one folder. [truncated] is true when the core cut the list short. */
data class DirListing(
    val entries: List<FileEntry>,
    val truncated: Boolean,
)

/** A file's text, read up to the core's cap. [text] is null for a binary file. */
data class FilePreview(
    val path: String,
    val sizeBytes: Long,
    val text: String?,
    val truncated: Boolean,
    val binary: Boolean,
)

/** One line that matched a text search. [line] is one-based. */
data class FileTextHit(
    val path: String,
    val line: Int,
    val text: String,
)

/** Why a files call failed, in the terms the screens need to explain it. */
enum class FilesProblem {
    /** The path is unsafe or names a file where a folder is needed. */
    InvalidPath,

    /** The project or file does not exist. */
    NotFound,

    /** The database or workspace cannot be read. */
    Unavailable,
}

/** A files call failed. [problem] says which message to show; [cause] is the core error. */
class FilesFailure(
    val problem: FilesProblem,
    cause: Throwable? = null,
) : Exception(cause)

/**
 * What the files screens need from the Rust core. Every call blocks, so callers run it off the
 * main thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface FilesGateway {
    /** The name of the project with [projectId], or null when none is saved under that id. */
    fun projectName(projectId: String): String?

    /** The children of [dir], where the empty string is the project root. */
    fun listDir(
        projectId: String,
        dir: String,
        showIgnored: Boolean,
    ): DirListing

    /** The text of the file at [path], read up to the core's cap. */
    fun readFile(
        projectId: String,
        path: String,
    ): FilePreview

    /** Project file paths whose name matches [pattern] (a glob or a substring), at most [limit]. */
    fun searchFileNames(
        projectId: String,
        pattern: String,
        limit: Int,
    ): List<String>

    /** Lines of indexed project files that contain [text], at most [limit]. */
    fun searchFileText(
        projectId: String,
        text: String,
        limit: Int,
    ): List<FileTextHit>
}

/**
 * The real gateway. It forwards each call to the UniFFI [FilesRuntime], and reads the project
 * name from [ProjectRuntime]. Both open on the first call, which runs off the main thread.
 */
class CoreFilesGateway(
    private val lazyFiles: Lazy<FilesRuntime>,
    private val lazyProjects: Lazy<ProjectRuntime>,
) : FilesGateway {
    private val files: FilesRuntime by lazyFiles
    private val projects: ProjectRuntime by lazyProjects

    override fun projectName(projectId: String): String? =
        try {
            projects.loadProject(projectId).name
        } catch (_: ProjectRuntimeException.NotFound) {
            null
        } catch (failure: ProjectRuntimeException) {
            throw FilesFailure(FilesProblem.Unavailable, failure)
        }

    override fun listDir(
        projectId: String,
        dir: String,
        showIgnored: Boolean,
    ): DirListing =
        guard {
            files.listDir(projectId, dir, showIgnored).let { view ->
                DirListing(
                    entries = view.entries.map { it.toEntry() },
                    truncated = view.truncated,
                )
            }
        }

    override fun readFile(
        projectId: String,
        path: String,
    ): FilePreview =
        guard {
            files.readFile(projectId, path).let { view ->
                FilePreview(
                    path = view.path,
                    sizeBytes = view.sizeBytes.toLong(),
                    text = view.text,
                    truncated = view.truncated,
                    binary = view.binary,
                )
            }
        }

    override fun searchFileNames(
        projectId: String,
        pattern: String,
        limit: Int,
    ): List<String> = guard { files.searchFileNames(projectId, pattern, limit.toUInt()) }

    override fun searchFileText(
        projectId: String,
        text: String,
        limit: Int,
    ): List<FileTextHit> =
        guard {
            files.searchFileText(projectId, text, limit.toUInt()).map {
                FileTextHit(path = it.path, line = it.line.toInt(), text = it.text)
            }
        }

    private fun FileEntryView.toEntry() =
        FileEntry(
            name = name,
            path = path,
            isDir = isDir,
            sizeBytes = sizeBytes.toLong(),
            ignored = ignored,
        )

    private inline fun <T> guard(block: () -> T): T =
        try {
            block()
        } catch (failure: FilesRuntimeException) {
            throw FilesFailure(failure.problem(), failure)
        }
}

/** The problem a core error stands for. */
internal fun FilesRuntimeException.problem(): FilesProblem =
    when (this) {
        is FilesRuntimeException.InvalidPath -> FilesProblem.InvalidPath
        is FilesRuntimeException.NotFound -> FilesProblem.NotFound
        is FilesRuntimeException.Unavailable -> FilesProblem.Unavailable
    }

/** The problem behind a failed files call. Any other failure counts as unavailable. */
internal fun Throwable.filesProblem(): FilesProblem =
    (this as? FilesFailure)?.problem ?: FilesProblem.Unavailable
