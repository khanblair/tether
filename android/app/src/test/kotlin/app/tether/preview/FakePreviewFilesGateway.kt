package app.tether.preview

import app.tether.files.FilesFailure
import app.tether.files.FilesProblem

/**
 * A preview files gateway for unit tests. Files are set by path. A read of a missing path throws
 * [FilesProblem.NotFound], and a path in [failing] throws [FilesProblem.Unavailable].
 */
class FakePreviewFilesGateway : PreviewFilesGateway {
    /** File bytes by path. */
    val files = mutableMapOf<String, ByteArray>()

    /** Paths whose reads fail as unavailable. */
    val failing = mutableSetOf<String>()

    /** Every read, as "path:limit", in order. */
    val reads = mutableListOf<String>()

    override fun readBytes(
        projectId: String,
        path: String,
        limitBytes: Long,
    ): FileBytes {
        reads += "$path:$limitBytes"
        if (path in failing) throw FilesFailure(FilesProblem.Unavailable)
        val bytes = files[path] ?: throw FilesFailure(FilesProblem.NotFound)
        val kept = minOf(bytes.size.toLong(), limitBytes).toInt()
        return FileBytes(bytes.copyOf(kept), bytes.size.toLong(), truncated = kept < bytes.size)
    }
}
