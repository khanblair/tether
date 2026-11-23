package app.tether.projectarchive

import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProjectRuntimeException

/**
 * What the archive screens need from the Rust core. Every call blocks, so callers run it off the
 * main thread. The fake in unit tests stands in for the native library.
 */
interface ProjectArchiveGateway {
    /** Unpacks the ZIP at [zipPath] into a new project named [name]. Returns the new project id. */
    fun import(
        zipPath: String,
        name: String,
    ): ArchiveResult<String>

    /** Cancels the import in flight. Does nothing when no import runs. */
    fun cancelImport()

    /** Writes project [projectId] as a ZIP at [destPath]. */
    fun export(
        projectId: String,
        destPath: String,
        includeIgnored: Boolean,
    ): ArchiveResult<Unit>
}

/**
 * The real gateway: forwards each call to the UniFFI [ProjectRuntime]. Core errors become
 * [ArchiveReason] values. Other exceptions are left for the view model to catch.
 */
class CoreProjectArchiveGateway(
    lazyRuntime: Lazy<ProjectRuntime>,
) : ProjectArchiveGateway {
    private val runtime: ProjectRuntime by lazyRuntime

    override fun import(
        zipPath: String,
        name: String,
    ): ArchiveResult<String> =
        try {
            ArchiveResult.Succeeded(runtime.importProjectZip(zipPath, name))
        } catch (error: ProjectRuntimeException) {
            ArchiveResult.Failed(error.toArchiveReason())
        }

    override fun cancelImport() {
        runtime.cancelImport()
    }

    override fun export(
        projectId: String,
        destPath: String,
        includeIgnored: Boolean,
    ): ArchiveResult<Unit> =
        try {
            runtime.exportProjectZip(projectId, destPath, includeIgnored)
            ArchiveResult.Succeeded(Unit)
        } catch (error: ProjectRuntimeException) {
            ArchiveResult.Failed(error.toArchiveReason())
        }
}

/** Builds the archive gateway over a runtime that opens on the first call, off the main thread. */
fun createProjectArchiveGateway(
    databasePath: String,
    workspacesRoot: String,
): ProjectArchiveGateway =
    CoreProjectArchiveGateway(
        lazy {
            ProjectRuntime(databasePath, workspacesRoot)
        },
    )
