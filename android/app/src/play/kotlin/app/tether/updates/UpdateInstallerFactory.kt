package app.tether.updates

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import androidx.core.net.toUri
import java.io.File

/** Creates the update installer for the Play build, which never installs updates itself. */
fun createUpdateInstaller(context: Context): UpdateInstaller =
    StoreLinkUpdateInstaller(context.applicationContext)

/** Play build installer: updates come from Google Play, so this only links to the store page. */
internal class StoreLinkUpdateInstaller(
    private val context: Context,
) : UpdateInstaller {
    override val installsInApp: Boolean = false

    override fun canRequestInstalls(): Boolean = false

    override fun openInstallPermissionSettings() = Unit

    override fun install(
        apk: File,
        listener: InstallListener,
    ) = listener.onFailure(InstallFailure.Blocked)

    override fun openStorePage() {
        val market =
            Intent(
                Intent.ACTION_VIEW,
                ("market://details?id=" + context.packageName).toUri(),
            ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        try {
            context.startActivity(market)
        } catch (e: ActivityNotFoundException) {
            val web =
                Intent(
                    Intent.ACTION_VIEW,
                    ("https://play.google.com/store/apps/details?id=" + context.packageName)
                        .toUri(),
                ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            context.startActivity(web)
        }
    }
}
