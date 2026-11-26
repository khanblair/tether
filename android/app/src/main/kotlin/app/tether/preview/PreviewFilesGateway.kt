package app.tether.preview

/** The bytes of one file. [sizeBytes] is the whole file; [truncated] is true when it was cut. */
class FileBytes(
    val bytes: ByteArray,
    val sizeBytes: Long,
    val truncated: Boolean,
)

/**
 * The bytes of project files for the preview WebView. A call blocks, so it runs off the main
 * thread. Errors use the same [app.tether.files.FilesFailure] problems as the files screens.
 */
interface PreviewFilesGateway {
    /** Reads up to [limitBytes] of the file at [path] in the project with [projectId]. */
    fun readBytes(
        projectId: String,
        path: String,
        limitBytes: Long,
    ): FileBytes
}
