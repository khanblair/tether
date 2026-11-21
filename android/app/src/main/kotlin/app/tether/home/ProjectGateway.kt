package app.tether.home

import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProjectRuntimeException
import app.tether.core.generated.ProjectView

/** A saved project as the app's screens use it. Times are epoch milliseconds. */
data class ProjectInfo(
    val id: String,
    val name: String,
    val lastActivityMs: Long,
)

/** The result of creating a project. */
sealed interface CreateProjectResult {
    /** The project now exists and survives a restart. */
    data class Created(
        val project: ProjectInfo,
    ) : CreateProjectResult

    /** The core rejected the name. */
    data object InvalidName : CreateProjectResult

    /** The project folder or the database could not be used. Nothing was created. */
    data object Unavailable : CreateProjectResult
}

/**
 * What the project screens need from the Rust core. Every call blocks, so callers run it off the
 * main thread. A fake implements this in unit tests, so no test loads the native library.
 */
interface ProjectGateway {
    /** Every saved project, in the core's order. */
    fun list(): List<ProjectInfo>

    /** Creates a project named [name]. The core trims the name and checks it again. */
    fun create(name: String): CreateProjectResult

    /** The project with [id], or null when none is saved under that id. */
    fun load(id: String): ProjectInfo?
}

/**
 * The real gateway: forwards each call to the UniFFI [ProjectRuntime]. The runtime is created on
 * the first call, which runs off the main thread, so opening the database never blocks the UI.
 */
class CoreProjectGateway(
    lazyRuntime: Lazy<ProjectRuntime>,
) : ProjectGateway {
    private val runtime: ProjectRuntime by lazyRuntime

    override fun list(): List<ProjectInfo> = runtime.listProjects().map(ProjectView::toInfo)

    override fun create(name: String): CreateProjectResult =
        try {
            CreateProjectResult.Created(runtime.createProject(name).toInfo())
        } catch (_: ProjectRuntimeException.InvalidName) {
            CreateProjectResult.InvalidName
        } catch (_: ProjectRuntimeException.Unavailable) {
            CreateProjectResult.Unavailable
        }

    override fun load(id: String): ProjectInfo? =
        try {
            runtime.loadProject(id).toInfo()
        } catch (_: ProjectRuntimeException.NotFound) {
            null
        }
}

private fun ProjectView.toInfo(): ProjectInfo =
    ProjectInfo(id = id, name = name, lastActivityMs = lastActivityMs)
