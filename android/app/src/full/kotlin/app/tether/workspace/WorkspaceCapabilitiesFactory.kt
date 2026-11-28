package app.tether.workspace

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Environment
import android.provider.Settings
import androidx.core.net.toUri

/** Creates the shared-folder capability for the full build. */
fun createWorkspaceCapabilities(): WorkspaceCapabilities = FullWorkspaceCapabilities

/**
 * The shared folder needs All files access, which only exists from Android 11 (API 30). The app
 * declares no legacy storage permission, so on Android 10 the option stays hidden instead of
 * offering a folder the app cannot write.
 */
private object FullWorkspaceCapabilities : WorkspaceCapabilities {
    override val sharedFolderAllowed: Boolean = Build.VERSION.SDK_INT >= Build.VERSION_CODES.R

    override fun allFilesAccessGranted(context: Context): Boolean =
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && Environment.isExternalStorageManager()

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
