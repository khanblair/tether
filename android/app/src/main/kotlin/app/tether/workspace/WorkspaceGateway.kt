package app.tether.workspace

import app.tether.core.generated.ProjectRuntime
import app.tether.core.generated.ProjectRuntimeException
import app.tether.core.generated.WorkspaceLocationView

/** The private default and the root used for new projects. */
data class WorkspaceLocation(
    val defaultRoot: String,
    val newProjectRoot: String?,
    val effectiveRoot: String,
)

/** The choice shown by the workspace screen. */
enum class WorkspaceChoice {
    Private,
    Custom,
}

/** A problem that the core can report for a selected workspace folder. */
sealed interface WorkspaceProblem {
    data object EmptyPath : WorkspaceProblem

    data object RootNotAbsolute : WorkspaceProblem

    data object RootCannotBeCreated : WorkspaceProblem

    data object RootNotDirectory : WorkspaceProblem

    data object RootIsSymlink : WorkspaceProblem

    data object RootNotWritable : WorkspaceProblem

    data object RootInsidePrivateRoot : WorkspaceProblem

    data object RootIsFilesystemRoot : WorkspaceProblem

    data object RootInsideProject : WorkspaceProblem

    data object RootUnavailable : WorkspaceProblem

    data object Unavailable : WorkspaceProblem
}

/** The workspace calls used by Settings and onboarding. */
interface WorkspaceGateway {
    /** Reads the saved location. The call blocks and must run off the main thread. */
    fun location(): WorkspaceLocation

    /** Saves the root for new projects, or returns the core validation problem. */
    fun setNewProjectRoot(path: String?): WorkspaceProblem?
}

/** Forwards workspace calls to the lazy UniFFI runtime. */
class CoreWorkspaceGateway(
    lazyRuntime: Lazy<ProjectRuntime>,
) : WorkspaceGateway {
    private val runtime: ProjectRuntime by lazyRuntime

    override fun location(): WorkspaceLocation = runtime.workspaceLocation().toWorkspaceLocation()

    override fun setNewProjectRoot(path: String?): WorkspaceProblem? =
        try {
            runtime.setNewProjectRoot(path)
            null
        } catch (error: ProjectRuntimeException) {
            error.toWorkspaceProblem()
        }
}

/** Maps every root error from the generated bindings to a screen problem. */
fun ProjectRuntimeException.toWorkspaceProblem(): WorkspaceProblem =
    when (this) {
        is ProjectRuntimeException.RootNotAbsolute -> WorkspaceProblem.RootNotAbsolute
        is ProjectRuntimeException.RootCannotBeCreated -> WorkspaceProblem.RootCannotBeCreated
        is ProjectRuntimeException.RootNotDirectory -> WorkspaceProblem.RootNotDirectory
        is ProjectRuntimeException.RootIsSymlink -> WorkspaceProblem.RootIsSymlink
        is ProjectRuntimeException.RootNotWritable -> WorkspaceProblem.RootNotWritable
        is ProjectRuntimeException.RootInsidePrivateRoot -> WorkspaceProblem.RootInsidePrivateRoot
        is ProjectRuntimeException.RootIsFilesystemRoot -> WorkspaceProblem.RootIsFilesystemRoot
        is ProjectRuntimeException.RootInsideProject -> WorkspaceProblem.RootInsideProject
        is ProjectRuntimeException.RootUnavailable -> WorkspaceProblem.RootUnavailable
        else -> WorkspaceProblem.Unavailable
    }

private fun WorkspaceLocationView.toWorkspaceLocation(): WorkspaceLocation =
    WorkspaceLocation(
        defaultRoot = defaultRoot,
        newProjectRoot = newProjectRoot,
        effectiveRoot = effectiveRoot,
    )
