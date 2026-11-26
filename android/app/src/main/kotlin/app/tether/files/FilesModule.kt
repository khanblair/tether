package app.tether.files

import app.tether.core.generated.FilesRuntime
import app.tether.core.generated.ProjectRuntime
import app.tether.preview.CorePreviewFilesGateway
import app.tether.preview.PreviewFilesGateway

/**
 * The files gateway over the app's database and workspaces. Both runtimes open on the first call,
 * off the main thread, so creating the gateway never blocks the UI.
 */
fun createFilesGateway(
    databasePath: String,
    workspacesRoot: String,
): FilesGateway =
    CoreFilesGateway(
        lazyFiles = lazy { FilesRuntime(databasePath, workspacesRoot) },
        lazyProjects = lazy { ProjectRuntime(databasePath, workspacesRoot) },
    )

/**
 * The gateway that the preview reads file bytes through. Its runtime opens on the first read, off
 * the main thread.
 */
fun createPreviewFilesGateway(
    databasePath: String,
    workspacesRoot: String,
): PreviewFilesGateway =
    CorePreviewFilesGateway(lazy { FilesRuntime(databasePath, workspacesRoot) })
