package app.tether.projectarchive

/**
 * An archive gateway for unit tests. It records each call and returns the result set on it. Set
 * [beforeImport] to run code inside the import call, such as a cancel that arrives mid-import.
 */
class FakeProjectArchiveGateway : ProjectArchiveGateway {
    /** One call to [import]. */
    data class ImportCall(
        val zipPath: String,
        val name: String,
    )

    /** One call to [export]. */
    data class ExportCall(
        val projectId: String,
        val destPath: String,
        val includeIgnored: Boolean,
    )

    val importCalls = mutableListOf<ImportCall>()
    val exportCalls = mutableListOf<ExportCall>()
    var cancelCount = 0

    /** What the next and later [import] calls return. */
    var importResult: ArchiveResult<String> = ArchiveResult.Succeeded("project-new")

    /** What the next and later [export] calls return. */
    var exportResult: ArchiveResult<Unit> = ArchiveResult.Succeeded(Unit)

    /** Runs inside [import], before the result is returned. */
    var beforeImport: (() -> Unit)? = null

    override fun import(
        zipPath: String,
        name: String,
    ): ArchiveResult<String> {
        importCalls += ImportCall(zipPath, name)
        beforeImport?.invoke()
        return importResult
    }

    override fun cancelImport() {
        cancelCount++
    }

    override fun export(
        projectId: String,
        destPath: String,
        includeIgnored: Boolean,
    ): ArchiveResult<Unit> {
        exportCalls += ExportCall(projectId, destPath, includeIgnored)
        return exportResult
    }
}
