package app.tether.preview

import app.tether.core.generated.FilesRuntime
import app.tether.core.generated.FilesRuntimeException
import app.tether.files.FilesFailure
import app.tether.files.problem

/**
 * The preview's file bytes from the Rust core. [files] opens on the first read, which runs off the
 * main thread. A core error becomes the same [FilesFailure] problem the files screens show.
 */
class CorePreviewFilesGateway(
    private val files: Lazy<FilesRuntime>,
) : PreviewFilesGateway {
    override fun readBytes(
        projectId: String,
        path: String,
        limitBytes: Long,
    ): FileBytes =
        try {
            files.value.readFileBytes(projectId, path, previewLimit(limitBytes)).let { view ->
                fileBytesOf(view.bytes, view.sizeBytes, view.truncated)
            }
        } catch (failure: FilesRuntimeException) {
            throw FilesFailure(failure.problem(), failure)
        }
}

/** The limit the core takes. A negative limit reads no bytes. */
internal fun previewLimit(limitBytes: Long): ULong = limitBytes.coerceAtLeast(0L).toULong()

/** A file's full size as a Long. A size past [Long.MAX_VALUE] is capped there. */
internal fun previewSize(sizeBytes: ULong): Long =
    sizeBytes.coerceAtMost(Long.MAX_VALUE.toULong()).toLong()

/** The preview's view of what the core read. */
internal fun fileBytesOf(
    bytes: ByteArray,
    sizeBytes: ULong,
    truncated: Boolean,
): FileBytes = FileBytes(bytes, previewSize(sizeBytes), truncated)
