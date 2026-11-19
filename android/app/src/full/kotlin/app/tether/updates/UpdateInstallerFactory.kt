package app.tether.updates

import android.content.Context

/** Creates the update installer for the full build, which installs updates in the app. */
fun createUpdateInstaller(context: Context): UpdateInstaller =
    PackageInstallerUpdateInstaller(context.applicationContext)
