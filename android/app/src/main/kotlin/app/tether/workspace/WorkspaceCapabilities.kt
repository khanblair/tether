package app.tether.workspace

import android.content.Context

/** Distribution-specific access to a shared workspace folder. */
interface WorkspaceCapabilities {
    /** True only in the full build. */
    val sharedFolderAllowed: Boolean

    /** Whether Android currently grants all-files access to this app. */
    fun allFilesAccessGranted(context: Context): Boolean

    /** Opens the system page where the person can grant all-files access. */
    fun openAllFilesAccessSettings(context: Context)
}
