package app.tether.workspace

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Environment
import android.provider.Settings
import androidx.core.net.toUri

/** Creates the shared-folder capability for the full build. */
fun createWorkspaceCapabilities(): WorkspaceCapabilities = FullWorkspaceCapabilities

private object FullWorkspaceCapabilities : WorkspaceCapabilities {
    override val sharedFolderAllowed: Boolean = true

    override fun allFilesAccessGranted(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.R || Environment.isExternalStorageManager()

    override fun openAllFilesAccessSettings(context: Context) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val intent =
                Intent(
                    Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION,
                    "package:${context.packageName}".toUri(),
                ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(intent)
        }
    }
}
