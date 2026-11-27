package app.tether.workspace

import android.content.Context

/** Creates the private-only capability for the Play build. */
fun createWorkspaceCapabilities(): WorkspaceCapabilities = PlayWorkspaceCapabilities

private object PlayWorkspaceCapabilities : WorkspaceCapabilities {
    override val sharedFolderAllowed: Boolean = false

    override fun allFilesAccessGranted(context: Context): Boolean = false

    override fun openAllFilesAccessSettings(context: Context) = Unit
}
