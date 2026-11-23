package app.tether.projectarchive

/**
 * Archive file access for unit tests. It invents cache paths, records each copy and delete, and
 * throws the exception set in [copyInFailure], [copyOutFailure], or [reserveFailure].
 */
class FakeArchiveFiles : ArchiveFiles {
    /** The size the provider reports for every URI. Null means unknown. */
    var sizeValue: Long? = 2_048L

    /** The display name the provider reports for every URI. */
    var nameValue: String? = "api-server.zip"

    var copyInFailure: Exception? = null
    var copyOutFailure: Exception? = null
    var reserveFailure: Exception? = null

    val copiedIn = mutableListOf<ArchiveUri>()
    val copiedOut = mutableListOf<Pair<TempFile, ArchiveUri>>()
    val deleted = mutableListOf<String>()
    private var counter = 0

    override fun copyIn(uri: ArchiveUri): TempFile {
        copyInFailure?.let { throw it }
        copiedIn += uri
        return TempFile("/cache/imports/zip-${++counter}.zip")
    }

    override fun reserveExport(): TempFile {
        reserveFailure?.let { throw it }
        return TempFile("/cache/exports/zip-${++counter}.zip")
    }

    override fun copyOut(
        temp: TempFile,
        uri: ArchiveUri,
    ) {
        copyOutFailure?.let { throw it }
        copiedOut += temp to uri
    }

    override fun delete(temp: TempFile) {
        deleted += temp.path
    }

    override fun displayName(uri: ArchiveUri): String? = nameValue

    override fun sizeBytes(uri: ArchiveUri): Long? = sizeValue
}
